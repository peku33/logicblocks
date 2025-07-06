pub mod event_source;
pub mod event_target_last;
pub mod event_target_queued;
pub mod state_source;
pub mod state_target_last;
pub mod state_target_queued;

use super::types::Base as ValueBase;
use std::{any::TypeId, fmt};

// Signals
pub trait Base: Send + Sync {
    fn as_remote_base(&self) -> &dyn RemoteBase;
}

pub trait StateSourceRemoteBase: RemoteBase {
    fn take_pending(&self) -> Box<[Option<Box<dyn ValueBase>>]>;
    fn peek_last(&self) -> Option<Box<dyn ValueBase>>;
}
pub trait StateTargetRemoteBase: RemoteBase {
    #[must_use = "use this value to wake signals change notifier"]
    fn set(
        &self,
        values: &[Option<Box<dyn ValueBase>>],
    ) -> bool;
}

pub trait EventSourceRemoteBase: RemoteBase {
    fn take_pending(&self) -> Box<[Box<dyn ValueBase>]>;
}
pub trait EventTargetRemoteBase: RemoteBase {
    #[must_use = "use this value to wake signals change notifier"]
    fn push(
        &self,
        values: &[Box<dyn ValueBase>],
    ) -> bool;
}

#[derive(Debug)]
pub enum RemoteBaseVariant<'a> {
    StateSource(&'a dyn StateSourceRemoteBase),
    StateTarget(&'a dyn StateTargetRemoteBase),
    EventSource(&'a dyn EventSourceRemoteBase),
    EventTarget(&'a dyn EventTargetRemoteBase),
}
pub trait RemoteBase: Send + Sync + fmt::Debug {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &'static str;

    fn as_remote_base_variant(&self) -> RemoteBaseVariant<'_>;
}
