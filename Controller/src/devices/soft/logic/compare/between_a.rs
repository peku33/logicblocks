use crate::{
    datatypes::range::Range,
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
pub struct Device<V>
where
    V: Value + Ord + Clone,
{
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_last::Signal<V>,
    signal_range: signal::state_target_last::Signal<Range<V>>,
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
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_range: signal::state_target_last::Signal::<Range<V>>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate_optional(
        input: &Option<V>,
        range: &Option<Range<V>>,
    ) -> Option<bool> {
        let input = match input {
            Some(input) => input,
            None => return None,
        };

        let range = match range {
            Some(range) => range,
            None => return None,
        };

        let value = Self::calculate(input, range);

        Some(value)
    }
    fn calculate(
        input: &V,
        range: &Range<V>,
    ) -> bool {
        range.contains(input)
    }

    fn signals_targets_changed(&self) {
        let mut signal_sources_changed = false;

        let value = Self::calculate_optional(
            &self.signal_input.take_last().value,
            &self.signal_range.take_last().value,
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
    Input,
    Range,
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
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Range => &self.signal_range as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
