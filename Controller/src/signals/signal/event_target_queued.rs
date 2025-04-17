use super::{
    super::types::{Base as ValueBase, event::Value},
    Base, EventTargetRemoteBase, RemoteBase, RemoteBaseVariant,
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

    pub fn take_pending(&self) -> Box<[V]> {
        let mut lock = self.inner.write();

        let pending = replace(&mut lock.pending, Vec::<V>::new());

        drop(lock);

        pending.into_boxed_slice()
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
    ) -> bool {
        let mut lock = self.inner.write();

        lock.pending.extend(
            values
                .iter()
                .map(|value| value.downcast_ref::<V>().unwrap().clone()),
        );

        drop(lock);

        true
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
