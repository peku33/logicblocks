use crate::{
    devices,
    signals::{self, signal},
    util::waker_stream,
};
use maplit::hashmap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Device {
    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::state_target_queued::Signal<bool>,
    signal_output: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new() -> Self {
        Self {
            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::state_target_queued::Signal::<bool>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/boolean/invert_a")
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        let values = self.signal_input.take_pending();

        let values = values
            .into_vec()
            .into_iter()
            .map(|value| value.map(|value| !value))
            .collect::<Box<[_]>>();

        if self.signal_output.set_many(values) {
            self.signal_sources_changed_waker.wake()
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
