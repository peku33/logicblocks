use super::{
    super::types::{Base as ValueBase, state::Value},
    Base, RemoteBase, RemoteBaseVariant, StateTargetRemoteBase,
};
use parking_lot::RwLock;
use std::{
    any::{TypeId, type_name},
    mem::replace,
};

#[derive(Debug)]
pub struct Last<V: Value + Clone> {
    pub value: Option<V>,
    pub pending: bool,
}

#[derive(Debug)]
struct Inner<V: Value + Clone> {
    last: Option<V>,
    pending: Vec<Option<V>>,
}

#[derive(Debug)]
pub struct Signal<V: Value + Clone> {
    inner: RwLock<Inner<V>>,
}
impl<V: Value + Clone> Signal<V> {
    pub fn new() -> Self {
        let inner = Inner {
            last: None,
            pending: Vec::<Option<V>>::new(),
        };

        Self {
            inner: RwLock::new(inner),
        }
    }

    // Clears pending flag
    // Returns pending if pending
    pub fn take_pending(&self) -> Box<[Option<V>]> {
        let mut lock = self.inner.write();

        let pending = replace(&mut lock.pending, Vec::<Option<V>>::new());

        drop(lock);

        pending.into_boxed_slice()
    }

    // Clears pending flag
    // Returns (last value, was pending)
    pub fn take_last(&self) -> Last<V> {
        let mut lock = self.inner.write();

        let value = lock.last.clone();
        let pending = !lock.pending.is_empty();
        lock.pending.clear();

        drop(lock);

        Last { value, pending }
    }

    // Does not clear pending flag
    // Returns last value
    pub fn peek_last(&self) -> Option<V> {
        let lock = self.inner.read();

        let value = lock.last.clone();

        drop(lock);

        value
    }
}
impl<V: Value + Clone> Base for Signal<V> {
    fn as_remote_base(&self) -> &dyn RemoteBase {
        self
    }
}
impl<V: Value + Clone> StateTargetRemoteBase for Signal<V> {
    // #[must_use = "use this value to wake signals change notifier"]
    fn set(
        &self,
        values: &[Option<Box<dyn ValueBase>>],
    ) -> bool {
        let mut lock = self.inner.write();

        let mut changes = false;

        for value in values {
            let value = value
                .as_ref()
                .map(|value| value.downcast_ref::<V>().unwrap().clone());

            if lock.last == value {
                continue;
            }

            lock.last.clone_from(&value);
            lock.pending.push(value);

            changes = true;
        }

        drop(lock);

        changes
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
        RemoteBaseVariant::StateTarget(self)
    }
}
