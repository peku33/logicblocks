use crate::{
    devices,
    signals::{self, signal, types::state::Value},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + Ord + Clone,
{
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_lower: signal::state_target_last::Signal<V>,
    signal_input: signal::state_target_last::Signal<V>,
    signal_upper: signal::state_target_last::Signal<V>,
    signal_output: signal::state_source::Signal<bool>,
}
impl<V> Device<V>
where
    V: Value + Ord + Clone,
{
    pub fn new() -> Self {
        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_lower: signal::state_target_last::Signal::<V>::new(),
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_upper: signal::state_target_last::Signal::<V>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate_outer(
        lower: Option<V>,
        input: Option<V>,
        upper: Option<V>,
    ) -> Option<bool> {
        match (lower, input, upper) {
            (Some(lower), Some(input), Some(upper)) => {
                Some(Self::calculate_inner(lower, input, upper))
            }
            _ => None,
        }
    }
    fn calculate_inner(
        lower: V,
        input: V,
        upper: V,
    ) -> bool {
        if lower <= upper {
            lower <= input && input <= upper
        } else {
            !(upper <= input && input <= lower)
        }
    }

    fn signals_targets_changed(&self) {
        let mut signal_sources_changed = false;

        let value = Self::calculate_outer(
            self.signal_lower.take_last().value,
            self.signal_input.take_last().value,
            self.signal_upper.take_last().value,
        );

        if self.signal_output.set_one(value) {
            signal_sources_changed = true;
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
            .stream(false)
            .stream_take_until_exhausted(exit_flag)
            .for_each(async move |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl<V> devices::Device for Device<V>
where
    V: Value + Ord + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!(
            "soft/logic/compare/between_a<{}>",
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
    V: Value + Ord + Clone,
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
    Lower,
    Input,
    Upper,
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: Value + Ord + Clone,
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
            SignalIdentifier::Lower => &self.signal_lower as &dyn signal::Base,
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Upper => &self.signal_upper as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
