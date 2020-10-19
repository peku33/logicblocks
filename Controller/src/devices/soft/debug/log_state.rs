use crate::{
    devices,
    signals::{self, signal, signal::state_target_queued, types::state::Value, Signals},
    util::waker_stream,
};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::{any::type_name, borrow::Cow};

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub name: String,
}

type SignalInput<V> = state_target_queued::Signal<V>;

#[derive(Debug)]
pub struct Device<V: Value + Clone> {
    configuration: Configuration,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: SignalInput<V>,

    gui_summary_waker: waker_stream::mpmc::Sender,
}
impl<V: Value + Clone> Device<V> {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: SignalInput::new(),

            gui_summary_waker: waker_stream::mpmc::Sender::new(),
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
    fn as_gui_summary_provider(&self) -> &dyn devices::GuiSummaryProvider {
        self
    }
}
impl<V: Value + Clone> signals::Device for Device<V> {
    fn signal_targets_changed_wake(&self) {
        let values = self.signal_input.take_pending();
        log::debug!("{}: {:?}", self.configuration.name, values);
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.signal_input as &dyn signal::Base,
        }
    }
}
impl<V: Value + Clone> devices::GuiSummaryProvider for Device<V> {
    fn get_value(&self) -> Box<dyn devices::GuiSummary> {
        Box::new(())
    }

    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
