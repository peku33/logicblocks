pub mod event_source;
pub mod event_target_last;
pub mod event_target_queued;
pub mod state_source;
pub mod state_target;

use super::types::Base as ValueBase;
use std::{any::TypeId, fmt};

// Signals
pub trait Base: Send + Sync {
    fn as_remote_base(&self) -> &dyn RemoteBase;
}

pub trait StateSourceRemoteBase: RemoteBase {
    fn take_pending(&self) -> Option<Box<dyn ValueBase>>;
    fn get_last(&self) -> Box<dyn ValueBase>;
}
pub trait StateTargetRemoteBase: RemoteBase {
    fn set(
        &self,
        value: &Option<Box<dyn ValueBase>>,
    ) -> bool;
}

pub trait EventSourceRemoteBase: RemoteBase {
    fn take_pending(&self) -> Box<[Box<dyn ValueBase>]>;
}
pub trait EventTargetRemoteBase: RemoteBase {
    fn push(
        &self,
        values: &[Box<dyn ValueBase>],
    );
}

pub enum RemoteBaseVariant<'a> {
    StateSource(&'a dyn StateSourceRemoteBase),
    StateTarget(&'a dyn StateTargetRemoteBase),
    EventSource(&'a dyn EventSourceRemoteBase),
    EventTarget(&'a dyn EventTargetRemoteBase),
}
pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &'static str;

    fn as_remote_base_variant(&self) -> RemoteBaseVariant;
}
