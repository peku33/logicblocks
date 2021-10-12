use crate::{
    devices,
    signals::{
        self, signal,
        types::{event::Value as EventValue, state::Value as StateValue},
    },
    util::waker_stream,
};
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Device<V>
where
    V: EventValue + StateValue + Clone,
{
    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::state_target_last::Signal<V>,
    signal_trigger: signal::event_target_last::Signal<()>,
    signal_output: signal::event_source::Signal<V>,
}
impl<V> Device<V>
where
    V: EventValue + StateValue + Clone,
{
    pub fn new() -> Self {
        Self {
            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_trigger: signal::event_target_last::Signal::<()>::new(),
            signal_output: signal::event_source::Signal::<V>::new(),
        }
    }
}
impl<V> devices::Device for Device<V>
where
    V: EventValue + StateValue + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/value/sample_a<{}>", type_name::<V>()))
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
impl<V> signals::Device for Device<V>
where
    V: EventValue + StateValue + Clone,
{
    fn signal_targets_changed_wake(&self) {
        let mut signal_sources_changed = false;

        if let Some(()) = self.signal_trigger.take_pending() {
            let value = self.signal_input.take_last().value;
            if let Some(value) = value {
                signal_sources_changed |= self.signal_output.push_one(value);
            }
        }

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
            1 => &self.signal_trigger as &dyn signal::Base,
            2 => &self.signal_output as &dyn signal::Base,
        }
    }
}
