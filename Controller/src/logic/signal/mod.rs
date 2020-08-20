pub mod event_source;
pub mod event_target;
pub mod state_source;
pub mod state_target;

use crate::datatypes::DataType;
use std::{any::Any, fmt};

pub trait ValueAny = Any + 'static + Send + Sync;

pub trait EventValue: ValueAny + fmt::Debug {}
impl<T> EventValue for T where T: DataType + ValueAny + fmt::Debug {}

pub trait StateValue: ValueAny + fmt::Debug {}
impl<T> StateValue for T where T: DataType + ValueAny + fmt::Debug {}

pub trait SignalBase {
    fn remote(&self) -> SignalRemoteBase;
}

pub enum SignalRemoteBase {
    StateSource(Box<dyn state_source::RemoteBase>),
    StateTarget(Box<dyn state_target::RemoteBase>),
    EventSource(Box<dyn event_source::RemoteBase>),
    EventTarget(Box<dyn event_target::RemoteBase>),
}
