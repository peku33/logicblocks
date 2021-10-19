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
    signal_output_raising: signal::event_source::Signal<()>,
    signal_output_falling: signal::event_source::Signal<()>,
}
impl Device {
    pub fn new() -> Self {
        Self {
            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::state_target_queued::Signal::<bool>::new(),
            signal_output_raising: signal::event_source::Signal::<()>::new(),
            signal_output_falling: signal::event_source::Signal::<()>::new(),
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/boolean/value/slope_a")
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        let mut raising = false;
        let mut falling = true;

        for value in self
            .signal_input
            .take_pending()
            .into_vec()
            .into_iter()
            .flatten()
        {
            if value {
                raising = true;
            } else {
                falling = true;
            }
        }

        let mut signal_sources_changed = false;

        if raising {
            signal_sources_changed |= self.signal_output_raising.push_one(());
        }
        if falling {
            signal_sources_changed |= self.signal_output_falling.push_one(());
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
            1 => &self.signal_output_raising as &dyn signal::Base,
            2 => &self.signal_output_falling as &dyn signal::Base,
        }
    }
}
