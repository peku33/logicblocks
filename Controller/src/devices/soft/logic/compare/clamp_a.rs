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
    signal_clamped: signal::state_source::Signal<V>, // clamped to range
    signal_checked: signal::state_source::Signal<V>, // = input if in range, none if outside range
    signal_status: signal::state_source::Signal<bool>, // true if inside range
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
            signal_clamped: signal::state_source::Signal::<V>::new(None),
            signal_checked: signal::state_source::Signal::<V>::new(None),
            signal_status: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate<'a>(
        input: &'a V,
        range: &'a Range<V>,
    ) -> (&'a V, Option<&'a V>, bool) {
        let clamped = range.clamp(input);
        let status = input == clamped;
        let checked = if status { Some(input) } else { None };

        (clamped, checked, status)
    }
    fn calculate_optional<'a>(
        input: &'a Option<V>,
        range: &'a Option<Range<V>>,
    ) -> (Option<&'a V>, Option<&'a V>, Option<bool>) {
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

        let input_last = self.signal_input.take_last();
        let range_last = self.signal_range.take_last();

        let (clamped, checked, status) =
            Self::calculate_optional(&input_last.value, &range_last.value);
        signals_sources_changed |= self.signal_clamped.set_one(clamped.cloned());
        signals_sources_changed |= self.signal_checked.set_one(checked.cloned());
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
    V: Value + Ord + Clone,
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
    Clamped,
    Checked,
    Status,
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
            SignalIdentifier::Clamped => &self.signal_clamped as &dyn signal::Base,
            SignalIdentifier::Checked => &self.signal_checked as &dyn signal::Base,
            SignalIdentifier::Status => &self.signal_status as &dyn signal::Base,
        }
    }
}
