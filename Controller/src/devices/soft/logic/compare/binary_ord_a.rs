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
use std::{any::type_name, borrow::Cow, iter};

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
        a: &V,
        b: &V,
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
pub struct Configuration<V>
where
    V: Value + PartialOrd + Clone,
{
    pub operation: Operation,
    pub a_fixed: Option<V>,
    pub b_fixed: Option<V>,
}

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + PartialOrd + Clone,
{
    configuration: Configuration<V>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_a: Option<signal::state_target_last::Signal<V>>,
    signal_b: Option<signal::state_target_last::Signal<V>>,
    signal_output: signal::state_source::Signal<bool>,
}
impl<V> Device<V>
where
    V: Value + PartialOrd + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        let a_fixed = configuration.a_fixed.is_some();
        let b_fixed = configuration.b_fixed.is_some();

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_a: if !a_fixed {
                Some(signal::state_target_last::Signal::<V>::new())
            } else {
                None
            },
            signal_b: if !b_fixed {
                Some(signal::state_target_last::Signal::<V>::new())
            } else {
                None
            },
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate(
        operation: &Operation,
        a: &V,
        b: &V,
    ) -> bool {
        operation.execute(a, b)
    }
    fn calculate_optional(
        operation: &Operation,
        a: Option<&V>,
        b: Option<&V>,
    ) -> Option<bool> {
        Some(Self::calculate(operation, a?, b?))
    }

    fn signals_targets_changed(&self) {
        let a = self
            .signal_a
            .as_ref()
            .and_then(|signal_a| signal_a.take_last().value);
        let a = match &self.configuration.a_fixed {
            Some(a_fixed) => Some(a_fixed),
            None => a.as_ref(),
        };

        let b = self
            .signal_b
            .as_ref()
            .and_then(|signal_b| signal_b.take_last().value);
        let b = match &self.configuration.b_fixed {
            Some(b_fixed) => Some(b_fixed),
            None => b.as_ref(),
        };

        let output = Self::calculate_optional(&self.configuration.operation, a, b);

        if self.signal_output.set_one(output) {
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
        iter::empty()
            .chain(self.signal_a.as_ref().map(|signal_a| {
                (
                    SignalIdentifier::A, // line break
                    signal_a as &dyn signal::Base,
                )
            }))
            .chain(self.signal_b.as_ref().map(|signal_b| {
                (
                    SignalIdentifier::B, // line break
                    signal_b as &dyn signal::Base,
                )
            }))
            .chain(iter::once((
                SignalIdentifier::Output,
                &self.signal_output as &dyn signal::Base,
            )))
            .collect::<signals::ByIdentifier<_>>()
    }
}
