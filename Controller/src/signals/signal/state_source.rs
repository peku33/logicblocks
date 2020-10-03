use super::{
    super::types::{state::Value, Base as ValueBase},
    Base, RemoteBase, RemoteBaseVariant, StateSourceRemoteBase,
};
use parking_lot::RwLock;
use std::any::{type_name, TypeId};

#[derive(Debug)]
struct ValuePending<V: Value + Clone> {
    value: V,
    pending: bool,
}

#[derive(Debug)]
pub struct Signal<V: Value + Clone> {
    value_pending: RwLock<ValuePending<V>>,
}
impl<V: Value + Clone> Signal<V> {
    pub fn new(initial: V) -> Self {
        Self {
            value_pending: RwLock::new(ValuePending {
                value: initial,
                pending: false,
            }),
        }
    }

    pub fn get(&self) -> V {
        self.value_pending.read().value.clone()
    }
    pub fn set(
        &self,
        value: V,
    ) -> bool {
        let mut lock = self.value_pending.write();
        if value == lock.value {
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
impl<V: Value + Clone> Base for Signal<V> {
    fn as_remote_base(&self) -> &dyn RemoteBase {
        self
    }
}

impl<V: Value + Clone> StateSourceRemoteBase for Signal<V> {
    fn take_pending(&self) -> Option<Box<dyn ValueBase>> {
        let mut lock = self.value_pending.write();
        if !lock.pending {
            return None;
        }
        let value = lock.value.clone();
        lock.pending = false;
        drop(lock);

        Some(Box::new(value))
    }

    fn get_last(&self) -> Box<dyn ValueBase> {
        let value = self.value_pending.read().value.clone();
        Box::new(value)
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
