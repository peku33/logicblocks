use crate::{
    devices,
    signals::{
        self,
        signal::{self, state_source, state_target},
        types::state::Value,
        Signals,
    },
    util::waker_stream,
    web::{sse_aggregated, uri_cursor},
};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration<V>
where
    V: Value + Clone,
{
    #[serde(bound = "")]
    pub default: V,
}

type SignalInput<V> = state_target::Signal<Option<V>>;
type SignalOutput<V> = state_source::Signal<V>;

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + Clone,
    Option<V>: Value + Clone,
{
    configuration: Configuration<V>,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: SignalInput<V>,
    signal_output: SignalOutput<V>,
}
impl<V> Device<V>
where
    V: Value + Clone,
    Option<V>: Value + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        let signal_output_value = configuration.default.clone();

        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: SignalInput::new(),
            signal_output: SignalOutput::new(signal_output_value),
        }
    }
}
impl<V> devices::Device for Device<V>
where
    V: Value + Clone,
    Option<V>: Value + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/value/or_default")
    }

    fn as_signals_device(&self) -> Option<&dyn signals::Device> {
        Some(self)
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        None
    }
    fn as_sse_aggregated_node_provider(&self) -> Option<&dyn sse_aggregated::NodeProvider> {
        None
    }
}
impl<V> signals::Device for Device<V>
where
    V: Value + Clone,
    Option<V>: Value + Clone,
{
    fn signal_targets_changed_wake(&self) {
        let value = match self.signal_input.take_pending() {
            Some(value) => value,
            None => return,
        };

        let value = value
            .unwrap_or_else(|| Some(self.configuration.default.clone()))
            .unwrap_or_else(|| self.configuration.default.clone());

        if self.signal_output.set(value) {
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
