use super::sse;
use crate::util::waker_stream::mpmc::ReceiverFactory;
use futures::stream::{SelectAll, Stream, StreamExt};
use std::{
    borrow::Cow,
    collections::{HashMap, LinkedList},
    sync::Arc,
};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
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

pub struct Node {
    pub terminal: Option<ReceiverFactory>,
    pub children: HashMap<PathItem, Node>,
}
pub trait NodeProvider {
    fn node(&self) -> Node;
}

fn build_recursive(node: Node) -> LinkedList<(ReceiverFactory, LinkedList<PathItem>)> {
    let mut paths = LinkedList::new();
    if let Some(terminal) = node.terminal {
        paths.push_back((terminal, LinkedList::new()));
    }
    for (path_item, child) in node.children {
        let mut child_paths = build_recursive(child);
        for (_, child_path) in child_paths.iter_mut() {
            child_path.push_front(path_item.clone());
        }
        paths.append(&mut child_paths);
    }
    paths
}

pub struct Bus {
    events: Vec<(ReceiverFactory, Arc<sse::Event>)>,
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
