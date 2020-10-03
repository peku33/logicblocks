use super::{
    super::types::{event::Value, Base as ValueBase},
    Base, EventTargetRemoteBase, RemoteBase, RemoteBaseVariant,
};
use crossbeam::queue::SegQueue;
use std::any::{type_name, TypeId};

#[derive(Debug)]
pub struct Signal<V: Value + Clone> {
    queue: SegQueue<V>,
}
impl<V: Value + Clone> Signal<V> {
    pub fn new() -> Self {
        Self {
            queue: SegQueue::new(),
        }
    }

    pub fn take_pending(&self) -> Box<[V]> {
        let mut buffer = Vec::with_capacity(self.queue.len());
        while let Ok(value) = self.queue.pop() {
            buffer.push(value);
        }
        buffer.into_boxed_slice()
    }
}
impl<V: Value + Clone> Base for Signal<V> {
    fn as_remote_base(&self) -> &dyn RemoteBase {
        self
    }
}

impl<V: Value + Clone> EventTargetRemoteBase for Signal<V> {
    fn push(
        &self,
        values: &[Box<dyn ValueBase>],
    ) {
        for value in values.iter() {
            let value = value.downcast_ref::<V>().unwrap().clone();
            self.queue.push(value);
        }
    }
}
impl<V: Value + Clone> RemoteBase for Signal<V> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<V>()
    }
    fn type_name(&self) -> &'static str {
        type_name::<V>()
    }

    fn as_remote_base_variant(&self) -> RemoteBaseVariant {
        RemoteBaseVariant::EventTarget(self)
    }
}
