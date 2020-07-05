use super::sse;
use crate::util::waker_stream;
use futures::stream::{SelectAll, Stream, StreamExt};
use std::{
    borrow::Cow,
    collections::{HashMap, LinkedList},
    sync::Arc,
};

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum PathItem {
    NumberU32(u32),
    String(String),
}
impl PathItem {
    pub fn into_json_value(self) -> serde_json::Value {
        match self {
            Self::NumberU32(value) => serde_json::Value::Number(value.into()),
            Self::String(string) => serde_json::Value::String(string),
        }
    }
}

pub enum Node {
    Terminal(waker_stream::ReceiverFactory),
    Children(HashMap<PathItem, Node>),
}
pub trait NodeProvider {
    fn node(&self) -> Node;
}

fn build_recursive(
    node: Node
) -> LinkedList<(waker_stream::ReceiverFactory, LinkedList<PathItem>)> {
    let mut paths = LinkedList::new();
    match node {
        Node::Terminal(receiver_factory) => {
            paths.push_back((receiver_factory, LinkedList::new()));
        }
        Node::Children(children_map) => {
            for (path_item, child) in children_map {
                let mut child_paths = build_recursive(child);
                for (_, child_path) in child_paths.iter_mut() {
                    child_path.push_front(path_item.clone());
                }
                paths.append(&mut child_paths);
            }
        }
    }
    paths
}

pub struct Bus {
    events: Vec<(waker_stream::ReceiverFactory, Arc<sse::Event>)>,
}
impl Bus {
    pub fn new(node: Node) -> Self {
        let events = build_recursive(node)
            .into_iter()
            .map(|(receiver_factory, path)| {
                let path_json = path
                    .into_iter()
                    .map(|item| item.into_json_value())
                    .collect::<Vec<serde_json::Value>>();
                let event = sse::Event {
                    id: None,
                    data: Cow::from(serde_json::to_string(&path_json).unwrap()),
                };
                let event = Arc::new(event);
                (receiver_factory, event)
            })
            .collect::<Vec<_>>();
        Self { events }
    }
    pub fn sse_stream(&self) -> impl Stream<Item = Arc<sse::Event>> + Send + 'static {
        self.events
            .iter()
            .map(move |(receiver_factory, event)| {
                let event = event.clone();
                receiver_factory.receiver().map(move |()| event.clone())
            })
            .collect::<SelectAll<_>>()
    }
}
