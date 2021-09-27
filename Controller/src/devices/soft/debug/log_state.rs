use crate::{
    devices,
    signals::{self, signal, types::state::Value},
    util::waker_stream,
};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::{any::type_name, borrow::Cow};

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub name: String,
}

#[derive(Debug)]
pub struct Device<V: Value + Clone> {
    configuration: Configuration,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::state_target_queued::Signal<V>,
}
impl<V: Value + Clone> Device<V> {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::state_target_queued::Signal::<V>::new(),
        }
    }
}
impl<V: Value + Clone> devices::Device for Device<V> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/debug/log_state<{}>", type_name::<V>()))
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
impl<V: Value + Clone> signals::Device for Device<V> {
    fn signal_targets_changed_wake(&self) {
        let values = self.signal_input.take_pending();
        log::info!("{}: {:?}", self.configuration.name, values);
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_input as &dyn signal::Base,
        }
    }
}
