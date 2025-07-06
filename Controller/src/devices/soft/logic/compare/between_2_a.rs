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
    pub range_false_fixed: Option<Range<V>>,
    pub range_true_fixed: Option<Range<V>>,
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
    signal_range_false: Option<signal::state_target_last::Signal<Range<V>>>,
    signal_range_true: Option<signal::state_target_last::Signal<Range<V>>>,
    signal_output: signal::state_source::Signal<bool>,
}
impl<V> Device<V>
where
    V: Value + PartialOrd + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        let range_false_fixed = configuration.range_false_fixed.is_some();
        let range_true_fixed = configuration.range_true_fixed.is_some();

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_range_false: if !range_false_fixed {
                Some(signal::state_target_last::Signal::<Range<V>>::new())
            } else {
                None
            },
            signal_range_true: if !range_true_fixed {
                Some(signal::state_target_last::Signal::<Range<V>>::new())
            } else {
                None
            },
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate(
        input: &V,
        range_false: &Range<V>,
        range_true: &Range<V>,
    ) -> Option<bool> {
        let value_false = range_false.contains(input);
        let value_true = range_true.contains(input);

        match (value_false, value_true) {
            (true, false) => Some(false),
            (false, true) => Some(true),
            _ => None,
        }
    }
    fn calculate_optional(
        input: Option<&V>,
        range_false: Option<&Range<V>>,
        range_true: Option<&Range<V>>,
    ) -> Option<bool> {
        Self::calculate(input?, range_false?, range_true?)
    }

    fn signals_targets_changed(&self) {
        let input = self.signal_input.take_last().value;
        let input = input.as_ref();

        let range_false = self
            .signal_range_false
            .as_ref()
            .and_then(|signal_range_false| signal_range_false.take_last().value);
        let range_false = match &self.configuration.range_false_fixed {
            Some(range_false_fixed) => Some(range_false_fixed),
            None => range_false.as_ref(),
        };

        let range_true = self
            .signal_range_true
            .as_ref()
            .and_then(|signal_range_true| signal_range_true.take_last().value);
        let range_true = match &self.configuration.range_true_fixed {
            Some(range_true_fixed) => Some(range_true_fixed),
            None => range_true.as_ref(),
        };

        let output = Self::calculate_optional(input, range_false, range_true);

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
            "soft/logic/compare/between_2_a<{}>",
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
    RangeFalse,
    RangeTrue,
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
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        chain!(
            iter::once((
                SignalIdentifier::Input,
                &self.signal_input as &dyn signal::Base,
            )),
            self.signal_range_false.as_ref().map(|signal_range_false| {
                (
                    SignalIdentifier::RangeFalse,
                    signal_range_false as &dyn signal::Base,
                )
            }),
            self.signal_range_true.as_ref().map(|signal_range_true| {
                (
                    SignalIdentifier::RangeTrue,
                    signal_range_true as &dyn signal::Base,
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
