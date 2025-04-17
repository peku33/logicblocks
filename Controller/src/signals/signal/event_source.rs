use super::{
    super::types::{Base as ValueBase, event::Value},
    Base, EventSourceRemoteBase, RemoteBase, RemoteBaseVariant,
};
use parking_lot::RwLock;
use std::{
    any::{TypeId, type_name},
    mem::replace,
};

#[derive(Debug)]
struct Inner<V: Value + Clone> {
    pending: Vec<V>,
}

#[derive(Debug)]
pub struct Signal<V: Value + Clone> {
    inner: RwLock<Inner<V>>,
}
impl<V: Value + Clone> Signal<V> {
    pub fn new() -> Self {
        let inner = Inner {
            pending: Vec::<V>::new(),
        };

        Self {
            inner: RwLock::new(inner),
        }
    }

    #[must_use = "use this value to wake signals change notifier"]
    pub fn push_one(
        &self,
        value: V,
    ) -> bool {
        let mut lock = self.inner.write();

        lock.pending.push(value);

        drop(lock);

        true
    }
    #[must_use = "use this value to wake signals change notifier"]
    pub fn push_many(
        &self,
        values: Box<[V]>,
    ) -> bool {
        let mut values = values.into_vec();

        let mut lock = self.inner.write();

        lock.pending.append(&mut values);

        drop(lock);

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
        let mut lock = self.inner.write();

        let pending = replace(&mut lock.pending, Vec::<V>::new());
        let pending = pending
            .into_iter()
            .map(|value| Box::new(value) as Box<dyn ValueBase>)
            .collect::<Box<[_]>>();

        drop(lock);

        pending
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
