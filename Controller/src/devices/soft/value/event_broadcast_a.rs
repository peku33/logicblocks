use crate::{
    devices,
    signals::{self, signal, types::event::Value},
    util::waker_stream,
};
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Device<V: Value + Clone> {
    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::event_target_queued::Signal<V>,
    signal_output: signal::event_source::Signal<V>,
}
impl<V: Value + Clone> Device<V> {
    pub fn new() -> Self {
        Self {
            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::event_target_queued::Signal::<V>::new(),
            signal_output: signal::event_source::Signal::<V>::new(),
        }
    }
}
impl<V: Value + Clone> devices::Device for Device<V> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!(
            "soft/value/event_broadcast_a<{}>",
            type_name::<V>()
        ))
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
impl<V: Value + Clone> signals::Device for Device<V> {
    fn signal_targets_changed_wake(&self) {
        let mut signal_sources_changed = false;

        signal_sources_changed |= self
            .signal_output
            .push_many(self.signal_input.take_pending());

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
        }
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_input as &dyn signal::Base,
            1 => &self.signal_output as &dyn signal::Base,
        }
    }
}
