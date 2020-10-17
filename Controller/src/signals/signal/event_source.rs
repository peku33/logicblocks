use super::{
    super::types::{event::Value, Base as ValueBase},
    Base, EventSourceRemoteBase, RemoteBase, RemoteBaseVariant,
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

    #[must_use = "use this value to wake signals change notifier"]
    pub fn push_one(
        &self,
        value: V,
    ) -> bool {
        self.queue.push(value);
        true
    }
    #[must_use = "use this value to wake signals change notifier"]
    pub fn push_many(
        &self,
        values: Box<[V]>,
    ) -> bool {
        for value in values.into_vec().into_iter() {
            self.queue.push(value);
        }
        true
    }
}
impl<V: Value + Clone> Base for Signal<V> {
    fn as_remote_base(&self) -> &dyn RemoteBase {
        self
    }
}
impl<V: Value + Clone> EventSourceRemoteBase for Signal<V> {
    fn take_pending(&self) -> Box<[Box<dyn ValueBase>]> {
        let mut buffer = Vec::with_capacity(self.queue.len());
        while let Ok(value) = self.queue.pop() {
            let value = Box::new(value) as Box<dyn ValueBase>;
            buffer.push(value);
        }
        buffer.into_boxed_slice()
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
        RemoteBaseVariant::EventSource(self)
    }
}
