use super::{Request, Response, sse, uri_cursor};
use crate::util::{
    async_ext::select_all_or_pending::{FutureSelectAllOrPending, StreamSelectAllOrPending},
    async_flag,
    async_waker::{mpmc_static, mpsc},
    runnable::{Exited, Runnable},
};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::{
    Stream,
    future::{BoxFuture, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

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
    inner: Box<[Topic]>,
}
impl TopicPath {
    pub fn new(inner: Box<[Topic]>) -> Self {
        Self { inner }
    }

    pub fn from_url_filter(value: &str) -> Option<Self> {
        let inner = value
            .split('-')
            .map(Topic::from_url_filter)
            .collect::<Option<_>>()?;
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
            .collect::<Option<_>>()?;
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
            data: Cow::from(self.to_sse_data().to_string()),
        }
    }
}

pub fn topic_paths_from_url_filter(value: &str) -> Option<HashSet<TopicPath>> {
    let topic_paths = value
        .split(',')
        .map(TopicPath::from_url_filter)
        .collect::<Option<_>>()?;
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
        .collect::<Option<_>>()?;
    Some(topic_paths)
}

#[derive(Debug)]
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

#[derive(Debug)]
struct ResponderTopicPathValue<'a> {
    waker: &'a mpsc::Signal,
    sender: mpmc_static::Sender,
    sse_event: sse::Event,
}
#[derive(Debug)]
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
        if let Some(self_) = &node.self_ {
            let topic_path = TopicPath::new(path.clone().into_boxed_slice());

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

        node.children.iter().for_each(|(topic, child)| {
            let mut path = path.clone();
            path.push(topic.clone());

            Self::traverse_node(topic_paths, path, child);
        });
    }

    fn make_topic_paths_stream_skip_missing(
        &self,
        topic_paths: &HashSet<TopicPath>,
    ) -> impl Stream<Item = sse::Event> + 'static {
        topic_paths
            .iter()
            .filter_map(|topic_path| {
                self.topic_paths
                    .get(topic_path)
                    .map(|value| (topic_path, value))
            })
            .map(|(_topic_path, value)| {
                let sse_event = value.sse_event.clone();
                value.sender.receiver().map(move |()| sse_event.clone())
            })
            .collect::<StreamSelectAllOrPending<_>>()
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let waker_to_sender_runner = self
            .topic_paths
            .values()
            .map(|value| {
                let receiver = value.waker.receiver();
                let sender = &value.sender;

                receiver.for_each(async move |_| sender.wake()).boxed()
            })
            .collect::<FutureSelectAllOrPending<_>>()
            .fuse();
        pin_mut!(waker_to_sender_runner);

        select! {
            _ = waker_to_sender_runner => panic!("waker_to_sender_runner yielded"),
            () = exit_flag => {},
        }

        Exited
    }
}
#[async_trait]
impl Runnable for Responder<'_> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl uri_cursor::Handler for Responder<'_> {
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
                            return async { Response::error_400_from_error(error) }.boxed();
                        }
                    };

                    let topic_paths = match topic_paths_from_url_filter(&filter_param)
                        .ok_or_else(|| anyhow!("failed to parse topic paths from url"))
                    {
                        Ok(topic_paths) => topic_paths,
                        Err(error) => {
                            return async { Response::error_400_from_error(error) }.boxed();
                        }
                    };

                    let topic_paths_stream =
                        self.make_topic_paths_stream_skip_missing(&topic_paths);

                    async { Response::ok_sse_stream(topic_paths_stream) }.boxed()
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
                            return async { Response::error_400_from_error(error) }.boxed();
                        }
                    };

                    let topic_paths_stream =
                        self.make_topic_paths_stream_skip_missing(&topic_paths);

                    async { Response::ok_sse_stream(topic_paths_stream) }.boxed()
                }
                _ => async { Response::error_405() }.boxed(),
            },
            _ => async { Response::error_404() }.boxed(),
        }
    }
}
