use crate::{
    devices,
    signals::{self, signal, types::state::Value},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
};
use async_trait::async_trait;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub enum Operation {
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
    LessOrEqual,
    Less,
}
impl Operation {
    pub fn execute<V: PartialEq + PartialOrd>(
        &self,
        a: V,
        b: V,
    ) -> bool {
        match self {
            Operation::Greater => a > b,
            Operation::GreaterOrEqual => a >= b,
            Operation::Equal => a == b,
            Operation::NotEqual => a != b,
            Operation::LessOrEqual => a <= b,
            Operation::Less => a < b,
        }
    }
}

#[derive(Debug)]
pub struct Configuration {
    pub operation: Operation,
}

#[derive(Debug)]
pub struct Device<V: Value + PartialEq + PartialOrd + Clone> {
    configuration: Configuration,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input_a: signal::state_target_last::Signal<V>,
    signal_input_b: signal::state_target_last::Signal<V>,
    signal_output: signal::state_source::Signal<bool>,
}
impl<V: Value + PartialEq + PartialOrd + Clone> Device<V> {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input_a: signal::state_target_last::Signal::new(),
            signal_input_b: signal::state_target_last::Signal::new(),
            signal_output: signal::state_source::Signal::new(None),
        }
    }
}
impl<V: Value + PartialEq + PartialOrd + Clone> devices::Device for Device<V> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!(
            "soft/logic/compare/binary_ord_a<{}>",
            type_name::<V>()
        ))
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
}
#[async_trait]
impl<V: Value + PartialEq + PartialOrd + Clone> Runnable for Device<V> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        exit_flag.await;
        Exited
    }
}
impl<V: Value + PartialEq + PartialOrd + Clone> signals::Device for Device<V> {
    fn signal_targets_changed_wake(&self) {
        let mut signal_sources_changed = false;

        let a = self.signal_input_a.take_last();
        let b = self.signal_input_b.take_last();
        if a.pending || b.pending {
            let output = match (a.value, b.value) {
                (Some(a), Some(b)) => Some(self.configuration.operation.execute(a, b)),
                _ => None,
            };
            signal_sources_changed |= self.signal_output.set_one(output);
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
            0 => &self.signal_output as &dyn signal::Base,
            1 => &self.signal_input_a as &dyn signal::Base,
            2 => &self.signal_input_b as &dyn signal::Base,
        }
    }
}
