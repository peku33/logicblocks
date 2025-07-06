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
    signal_clamped: signal::state_source::Signal<V>, // clamped to range
    signal_checked: signal::state_source::Signal<V>, // = input if in range, none if outside range
    signal_status: signal::state_source::Signal<bool>, // true if inside range
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
            signal_clamped: signal::state_source::Signal::<V>::new(None),
            signal_checked: signal::state_source::Signal::<V>::new(None),
            signal_status: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate(
        input: V,
        range: &Range<V>,
    ) -> (V, Option<V>, bool) {
        let clamped = range.clamp_to(input.clone());
        let status = input == clamped;
        let checked = if status { Some(input) } else { None };

        (clamped, checked, status)
    }
    fn calculate_optional(
        input: Option<V>,
        range: Option<&Range<V>>,
    ) -> (Option<V>, Option<V>, Option<bool>) {
        match (input, range) {
            (Some(input), Some(range)) => {
                let (clamped, checked, status) = Self::calculate(input, range);
                (Some(clamped), checked, Some(status))
            }
            _ => (None, None, None),
        }
    }

    fn signals_targets_changed(&self) {
        let mut signals_sources_changed = false;

        let input = self.signal_input.take_last().value;

        let range = self
            .signal_range
            .as_ref()
            .and_then(|signal_range| signal_range.take_last().value);
        let range = match &self.configuration.range_fixed {
            Some(range_fixed) => Some(range_fixed),
            None => range.as_ref(),
        };

        let (clamped, checked, status) = Self::calculate_optional(input, range);

        signals_sources_changed |= self.signal_clamped.set_one(clamped);
        signals_sources_changed |= self.signal_checked.set_one(checked);
        signals_sources_changed |= self.signal_status.set_one(status);

        if signals_sources_changed {
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
        Cow::from(format!("soft/logic/compare/clamp_a<{}>", type_name::<V>()))
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
    Clamped,
    Checked,
    Status,
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
            self.signal_range.as_ref().map(|signal_range| {
                (
                    SignalIdentifier::Range, // line break
                    signal_range as &dyn signal::Base,
                )
            }),
            iter::once((
                SignalIdentifier::Clamped,
                &self.signal_clamped as &dyn signal::Base,
            )),
            iter::once((
                SignalIdentifier::Checked,
                &self.signal_checked as &dyn signal::Base,
            )),
            iter::once((
                SignalIdentifier::Status,
                &self.signal_status as &dyn signal::Base,
            )),
        )
        .collect::<signals::ByIdentifier<_>>()
    }
}
