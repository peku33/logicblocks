use super::{
    super::types::{Base as ValueBase, event::Value},
    Base, EventTargetRemoteBase, RemoteBase, RemoteBaseVariant,
};
use parking_lot::RwLock;
use std::any::{TypeId, type_name};

#[derive(Debug)]
pub struct Signal<V: Value + Clone> {
    pending: RwLock<Option<V>>,
}
impl<V: Value + Clone> Signal<V> {
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(None),
        }
    }

    pub fn take_pending(&self) -> Option<V> {
        self.pending.write().take()
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
        let value = match values.iter().last() {
            Some(value) => value,
            None => return false,
        };
        let value = value.downcast_ref::<V>().unwrap().clone();
        *self.pending.write() = Some(value);
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
        RemoteBaseVariant::EventTarget(self)
    }
}
