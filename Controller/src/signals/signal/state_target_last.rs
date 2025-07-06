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
struct ValuePending<V: Value + Clone> {
    value: Option<V>,
    pending: bool,
}

#[derive(Debug)]
pub struct Signal<V: Value + Clone> {
    value_pending: RwLock<ValuePending<V>>,
}
impl<V: Value + Clone> Signal<V> {
    pub fn new() -> Self {
        Self {
            value_pending: RwLock::new(ValuePending {
                value: None,
                pending: false,
            }),
        }
    }

    // Clears pending flag
    // Returns pending if pending
    pub fn take_pending(&self) -> Option<Option<V>> {
        let mut lock = self.value_pending.write();

        if !lock.pending {
            return None;
        }
        let value = lock.value.clone();
        lock.pending = false;
        drop(lock);

        Some(value)
    }

    // Clears pending flag
    // Returns (last value, was pending)
    pub fn take_last(&self) -> Last<V> {
        let mut lock = self.value_pending.write();

        let value = lock.value.clone();
        let pending = replace(&mut lock.pending, false);
        drop(lock);

        Last { value, pending }
    }

    // Does not clear pending flag
    // Returns last value
    pub fn peek_last(&self) -> Option<V> {
        self.value_pending.read().value.clone()
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
        let value = match values.iter().last() {
            Some(value) => value,
            None => return false,
        };
        let value = value
            .as_ref()
            .map(|value| value.downcast_ref::<V>().unwrap().clone());

        let mut lock = self.value_pending.write();

        if lock.value == value {
            return false;
        }

        *lock = ValuePending {
            value,
            pending: true,
        };

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

    fn as_remote_base_variant(&self) -> RemoteBaseVariant<'_> {
        RemoteBaseVariant::StateTarget(self)
    }
}
