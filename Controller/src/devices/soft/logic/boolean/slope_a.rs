use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
};
use async_trait::async_trait;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize, Debug)]
pub enum Edge {
    Raising,
    Falling,
    Both,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub edge: Edge,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::state_target_queued::Signal<bool>,
    signal_output: signal::event_source::Signal<()>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::state_target_queued::Signal::<bool>::new(),
            signal_output: signal::event_source::Signal::<()>::new(),
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/boolean/slope_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
#[async_trait]
impl Runnable for Device {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        exit_flag.await;
        Exited
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        let values = self.signal_input.take_pending();

        let values = values
            .into_vec()
            .into_iter()
            .flatten()
            .filter_map(|value| match (value, &self.configuration.edge) {
                (true, Edge::Raising) => Some(()),
                (false, Edge::Falling) => Some(()),
                (_, Edge::Both) => Some(()),
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
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_input as &dyn signal::Base,
            1 => &self.signal_output as &dyn signal::Base,
        }
    }
}
