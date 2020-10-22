pub mod exchange;
pub mod signal;
pub mod types;

use crate::util::waker_stream;
use std::collections::HashMap;

pub type Id = u16;

pub type Signals<'s> = HashMap<Id, &'s dyn signal::Base>;

pub trait Device: Send + Sync {
    fn signal_targets_changed_wake(&self);
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease;
    fn signals(&self) -> Signals;
}
