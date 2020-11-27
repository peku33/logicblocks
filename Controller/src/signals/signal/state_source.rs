use super::{
    super::types::{state::Value, Base as ValueBase},
    Base, RemoteBase, RemoteBaseVariant, StateSourceRemoteBase,
};
use parking_lot::RwLock;
use std::{
    any::{type_name, TypeId},
    mem::replace,
};

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
    pub fn new(initial: Option<V>) -> Self {
        let inner = Inner {
            last: initial,
            pending: Vec::new(),
        };

        Self {
            inner: RwLock::new(inner),
        }
    }

    pub fn get_last(&self) -> Option<V> {
        self.inner.read().last.clone()
    }

    #[must_use = "use this value to wake signals change notifier"]
    pub fn set_one(
        &self,
        value: Option<V>,
    ) -> bool {
        let mut lock = self.inner.write();

        if lock.last == value {
            return false;
        }
        lock.last = value.clone();
        lock.pending.push(value);

        drop(lock);

        true
    }
    #[must_use = "use this value to wake signals change notifier"]
    pub fn set_many(
        &self,
        values: Box<[Option<V>]>,
    ) -> bool {
        if values.is_empty() {
            return false;
        }

        let mut changes = false;

        let mut lock = self.inner.write();

        for value in values.into_vec().into_iter() {
            if lock.last == value {
                continue;
            }

            lock.last = value.clone();
            lock.pending.push(value);

            changes = true;
        }

        drop(lock);

        changes
    }
}
impl<V: Value + Clone> Base for Signal<V> {
    fn as_remote_base(&self) -> &dyn RemoteBase {
        self
    }
}
impl<V: Value + Clone> StateSourceRemoteBase for Signal<V> {
    fn take_pending(&self) -> Box<[Option<Box<dyn ValueBase>>]> {
        let mut lock = self.inner.write();

        let pending = replace(&mut lock.pending, Vec::new());
        let pending = pending
            .into_iter()
            .map(|value| value.map(|value| Box::new(value) as Box<dyn ValueBase>))
            .collect::<Box<[_]>>();

        drop(lock);

        pending
    }

    fn get_last(&self) -> Option<Box<dyn ValueBase>> {
        self.inner
            .read()
            .last
            .clone()
            .map(|value| Box::new(value) as Box<dyn ValueBase>)
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
        RemoteBaseVariant::StateSource(self)
    }
}
