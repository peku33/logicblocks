use crate::{
    devices,
    signals::{
        self,
        signal::{self, event_source, state_target_queued},
        Signals,
    },
    util::waker_stream,
};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

type SignalInput = state_target_queued::Signal<bool>;
type SignalOutput = event_source::Signal<()>;

#[derive(Serialize, Deserialize, Debug)]
pub enum Edge {
    RAISING,
    FALLING,
    BOTH,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub edge: Edge,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: SignalInput,
    signal_output: SignalOutput,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: SignalInput::new(),
            signal_output: SignalOutput::new(),
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/boolean/slope_a")
    }

    fn as_signals_device(&self) -> Option<&dyn signals::Device> {
        Some(self)
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        let values = self.signal_input.take_pending();

        let values = values
            .into_vec()
            .into_iter()
            .filter_map(|value| value)
            .filter_map(|value| match (value, &self.configuration.edge) {
                (true, Edge::RAISING) => Some(()),
                (false, Edge::FALLING) => Some(()),
                (_, Edge::BOTH) => Some(()),
                _ => None,
            })
            .collect::<Box<[_]>>();

        if self.signal_output.push_many(values) {
            self.signal_sources_changed_waker.wake();
        }
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.signal_input as &dyn signal::Base,
            1 => &self.signal_output as &dyn signal::Base,
        }
    }
}
