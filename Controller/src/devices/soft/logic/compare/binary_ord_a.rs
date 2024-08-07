use crate::{
    devices,
    signals::{self, signal, types::state::Value},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
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
    pub fn execute<V>(
        &self,
        a: V,
        b: V,
    ) -> bool
    where
        V: PartialOrd,
    {
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
pub struct Device<V>
where
    V: Value + PartialOrd + Clone,
{
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_a: signal::state_target_last::Signal<V>,
    signal_b: signal::state_target_last::Signal<V>,
    signal_output: signal::state_source::Signal<bool>,
}
impl<V> Device<V>
where
    V: Value + PartialOrd + Clone,
{
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_a: signal::state_target_last::Signal::<V>::new(),
            signal_b: signal::state_target_last::Signal::<V>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn signals_targets_changed(&self) {
        let mut signal_sources_changed = false;

        let a = self.signal_a.take_last();
        let b = self.signal_b.take_last();
        if a.pending || b.pending {
            let output = match (a.value, b.value) {
                (Some(a), Some(b)) => Some(self.configuration.operation.execute(a, b)),
                _ => None,
            };
            if self.signal_output.set_one(output) {
                signal_sources_changed = true;
            }
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.signals_targets_changed_waker
            .stream()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl<V> devices::Device for Device<V>
where
    V: Value + PartialOrd + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!(
            "soft/logic/compare/binary_ord_a<{}>",
            type_name::<V>()
        ))
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
}

#[async_trait]
impl<V> Runnable for Device<V>
where
    V: Value + PartialOrd + Clone,
{
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    A,
    B,
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: Value + PartialOrd + Clone,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
        hashmap! {
            SignalIdentifier::A => &self.signal_a as &dyn signal::Base,
            SignalIdentifier::B => &self.signal_b as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
