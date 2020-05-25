pub mod event_source;
pub mod event_target;
pub mod state_source;
pub mod state_target;

use std::{any::Any, fmt};

pub trait Value: Any + Send + Sync + fmt::Debug + 'static {}

pub trait StateValue: Value {}
pub trait EventValue: Value {}

pub trait ValueAny = Any + 'static + Send + Sync;

pub trait SignalBase {
    fn remote(&self) -> SignalRemoteBase;
}

pub enum SignalRemoteBase {
    StateSource(Box<dyn state_source::RemoteBase>),
    StateTarget(Box<dyn state_target::RemoteBase>),
    EventSource(Box<dyn event_source::RemoteBase>),
    EventTarget(Box<dyn event_target::RemoteBase>),
}
