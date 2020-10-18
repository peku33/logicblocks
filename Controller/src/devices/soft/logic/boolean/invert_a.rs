use crate::{
    devices,
    signals::{
        self,
        signal::{self, state_source, state_target_queued},
        Signals,
    },
    util::waker_stream,
};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

type SignalInput = state_target_queued::Signal<bool>;
type SignalOutput = state_source::Signal<bool>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub default_state: bool,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: SignalInput,
    signal_output: SignalOutput,

    gui_summary_provider_waker: waker_stream::mpmc::Sender,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let default_state = configuration.default_state;

        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: SignalInput::new(),
            signal_output: SignalOutput::new(default_state),

            gui_summary_provider_waker: waker_stream::mpmc::Sender::new(),
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
    fn as_gui_summary_provider(&self) -> &dyn devices::GuiSummaryProvider {
        self
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        let values = self.signal_input.take_pending();

        let values = values
            .into_vec()
            .into_iter()
            .map(|value| match value {
                Some(value) => !value,
                None => self.configuration.default_state,
            })
            .collect::<Box<[_]>>();

        if self.signal_output.set_many(values) {
            self.signal_sources_changed_waker.wake()
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
impl devices::GuiSummaryProvider for Device {
    fn get_value(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_provider_waker.receiver_factory()
    }
}
