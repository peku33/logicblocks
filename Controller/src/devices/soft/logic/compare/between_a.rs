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
use itertools::chain;
use std::{any::type_name, borrow::Cow, iter};

#[derive(Debug)]
pub struct Configuration<V>
where
    V: Value + PartialOrd + Clone,
{
    pub range_fixed: Option<Range<V>>,
}

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + PartialOrd + Clone,
{
    configuration: Configuration<V>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_last::Signal<V>,
    signal_range: Option<signal::state_target_last::Signal<Range<V>>>,
    signal_output: signal::state_source::Signal<bool>,
}
impl<V> Device<V>
where
    V: Value + PartialOrd + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        let range_fixed = configuration.range_fixed.is_some();

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_range: if !range_fixed {
                Some(signal::state_target_last::Signal::<Range<V>>::new())
            } else {
                None
            },
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate(
        input: &V,
        range: &Range<V>,
    ) -> bool {
        range.contains(input)
    }
    fn calculate_optional(
        input: Option<&V>,
        range: Option<&Range<V>>,
    ) -> Option<bool> {
        Some(Self::calculate(input?, range?))
    }

    fn signals_targets_changed(&self) {
        let input = self.signal_input.take_last().value;
        let input = input.as_ref();

        let range = self
            .signal_range
            .as_ref()
            .and_then(|signal_range| signal_range.take_last().value);
        let range = match &self.configuration.range_fixed {
            Some(range_fixed) => Some(range_fixed),
            None => range.as_ref(),
        };

        let output = Self::calculate_optional(input, range);

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
    Input,
    Range,
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
        chain!(
            iter::once((
                SignalIdentifier::Input,
                &self.signal_input as &dyn signal::Base,
            )),
            self.signal_range.as_ref().map(|signal_range| {
                (
                    SignalIdentifier::Range, // line break
                    signal_range as &dyn signal::Base,
                )
            }),
            iter::once((
                SignalIdentifier::Output,
                &self.signal_output as &dyn signal::Base,
            )),
        )
        .collect::<signals::ByIdentifier<_>>()
    }
}
