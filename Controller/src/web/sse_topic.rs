use super::{sse, uri_cursor, Request, Response};
use crate::util::{
    async_flag,
    runtime::{Exited, Runnable},
    waker_stream::{mpmc_static, mpsc},
};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt, JoinAll},
    pin_mut, select,
    stream::{pending, select, SelectAll, StreamExt},
    Stream,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Topic {
    Number(usize),
    String(String),
}
impl Topic {
    pub fn from_url_filter(value: &str) -> Option<Self> {
        // for now only Number() is supported
        if let Ok(value) = str::parse::<usize>(value) {
            return Some(Self::Number(value));
        }

        None
    }
    pub fn from_body_filter(value: serde_json::Value) -> Option<Self> {
        let value = match value {
            serde_json::Value::Number(value) => match value.as_u64() {
                Some(value) => Self::Number(value as usize),
                None => return None,
            },
            serde_json::Value::String(value) => Self::String(value),
            _ => return None,
        };

        Some(value)
    }
    pub fn to_sse_data(&self) -> serde_json::Value {
        match self {
            Self::Number(value) => serde_json::Value::Number((*value).into()),
            Self::String(value) => serde_json::Value::String(value.clone()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TopicPath {
    inner: Vec<Topic>,
}
impl TopicPath {
    pub fn from_inner(inner: Vec<Topic>) -> Self {
        Self { inner }
    }

    pub fn from_url_filter(value: &str) -> Option<Self> {
        let inner = value
            .split('-')
            .map(Topic::from_url_filter)
            .collect::<Option<Vec<_>>>()?;
        let self_ = Self { inner };
        Some(self_)
    }
    pub fn from_body_filter(value: serde_json::Value) -> Option<Self> {
        let value = match value {
            serde_json::Value::Array(value) => value,
            _ => return None,
        };

        let inner = value
            .into_iter()
            .map(Topic::from_body_filter)
            .collect::<Option<Vec<_>>>()?;
        let self_ = Self { inner };
        Some(self_)
    }
    pub fn to_sse_data(&self) -> serde_json::Value {
        serde_json::Value::Array(
            self.inner
                .iter()
                .map(|topic| topic.to_sse_data())
                .collect::<Vec<_>>(),
        )
    }
    pub fn to_sse_event(&self) -> sse::Event {
        sse::Event {
            id: None,
            data: self.to_sse_data().to_string().into(),
        }
    }
}

pub fn topic_paths_from_url_filter(value: &str) -> Option<HashSet<TopicPath>> {
    let topic_paths = value
        .split(',')
        .map(TopicPath::from_url_filter)
        .collect::<Option<HashSet<_>>>()?;
    Some(topic_paths)
}
pub fn topic_paths_from_body_filter(value: serde_json::Value) -> Option<HashSet<TopicPath>> {
    let value = match value {
        serde_json::Value::Array(value) => value,
        _ => return None,
    };

    let topic_paths = value
        .into_iter()
        .map(TopicPath::from_body_filter)
        .collect::<Option<HashSet<_>>>()?;
    Some(topic_paths)
}

pub struct Node<'a> {
    self_: Option<&'a mpsc::Signal>,
    children: HashMap<Topic, Node<'a>>,
}
impl<'a> Node<'a> {
    pub fn new(
        self_: Option<&'a mpsc::Signal>,
        children: HashMap<Topic, Node<'a>>,
    ) -> Self {
        Self { self_, children }
    }
}

struct ResponderTopicPathValue<'a> {
    waker: &'a mpsc::Signal,
    sender: mpmc_static::Sender,
    sse_event: sse::Event,
}
pub struct Responder<'a> {
    root: &'a Node<'a>,
    topic_paths: HashMap<TopicPath, ResponderTopicPathValue<'a>>,
}
impl<'a> Responder<'a> {
    pub fn new(root: &'a Node<'a>) -> Self {
        let mut topic_paths = HashMap::<TopicPath, ResponderTopicPathValue<'a>>::new();
        Self::traverse_node(&mut topic_paths, Vec::new(), root);

        Self { root, topic_paths }
    }

    fn traverse_node(
        topic_paths: &mut HashMap<TopicPath, ResponderTopicPathValue<'a>>,
        path: Vec<Topic>,
        node: &'a Node<'a>,
    ) {
        if let Some(self_) = node.self_.as_ref() {
            let topic_path = TopicPath::from_inner(path.clone());

            let waker = self_;
            let sender = mpmc_static::Sender::new();
            let sse_event = topic_path.to_sse_event();

            let value = ResponderTopicPathValue {
                waker,
                sender,
                sse_event,
            };

            let inserted = topic_paths.insert(topic_path, value).is_none();
            debug_assert!(inserted);
        }

        for (topic, child) in &node.children {
            let mut path = path.clone();
            path.push(topic.clone());

            Self::traverse_node(topic_paths, path, child);
        }
    }

    fn make_topic_paths_stream_skip_missing(
        &self,
        topic_paths: &HashSet<TopicPath>,
    ) -> impl Stream<Item = sse::Event> + 'static {
        let stream = topic_paths
            .iter()
            .filter_map(|topic_path| {
                self.topic_paths
                    .get(topic_path)
                    .map(|value| (topic_path, value))
            })
            .map(|(_topic_path, value)| {
                let sse_event = value.sse_event.clone();
                let stream = value.sender.receiver().map(move |()| sse_event.clone());
                stream
            })
            .collect::<SelectAll<_>>();

        // make the stream infinite
        select(stream, pending())
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let waker_to_sender_runner = self
            .topic_paths
            .iter()
            .map(async move |(_topic_path, value)| {
                let receiver = value.waker.receiver();
                let sender = &value.sender;

                receiver.for_each(async move |()| sender.wake()).await;
            })
            .collect::<JoinAll<_>>();
        pin_mut!(waker_to_sender_runner);
        let mut waker_to_sender_runner = waker_to_sender_runner.fuse();

        select! {
            _ = waker_to_sender_runner => panic!("waker_to_sender_runner yielded"),
            () = exit_flag => {},
        }

        Exited
    }
}
#[async_trait]
impl<'a> Runnable for Responder<'a> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl<'a> uri_cursor::Handler for Responder<'a> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::GET => {
                    let filter_param = match form_urlencoded::parse(
                        request.uri().query().unwrap_or("").as_bytes(),
                    )
                    .find_map(|(key, value)| {
                        if key == "filter" {
                            Some(value.into_owned())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| anyhow!("missing filter parameter"))
                    {
                        Ok(filter_param) => filter_param,
                        Err(error) => {
                            return async move { Response::error_400_from_error(error) }.boxed()
                        }
                    };

                    let topic_paths = match topic_paths_from_url_filter(&filter_param)
                        .ok_or_else(|| anyhow!("failed to parse topic paths from url"))
                    {
                        Ok(topic_paths) => topic_paths,
                        Err(error) => {
                            return async move { Response::error_400_from_error(error) }.boxed()
                        }
                    };

                    let topic_paths_stream =
                        self.make_topic_paths_stream_skip_missing(&topic_paths);

                    async move { Response::ok_sse_stream(topic_paths_stream) }.boxed()
                }
                http::Method::POST => {
                    let topic_paths = match request
                        .body_parse_json::<serde_json::Value>()
                        .ok()
                        .and_then(topic_paths_from_body_filter)
                        .ok_or_else(|| anyhow!("failed to parse topic paths from body"))
                    {
                        Ok(topic_paths) => topic_paths,
                        Err(error) => {
                            return async move { Response::error_400_from_error(error) }.boxed()
                        }
                    };

                    let topic_paths_stream =
                        self.make_topic_paths_stream_skip_missing(&topic_paths);

                    async move { Response::ok_sse_stream(topic_paths_stream) }.boxed()
                }
                _ => async move { Response::error_405() }.boxed(),
            },
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
