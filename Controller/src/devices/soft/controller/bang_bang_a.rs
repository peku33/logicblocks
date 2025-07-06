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
use itertools::chain;
use std::{any::type_name, borrow::Cow, cmp::Ordering, iter};

#[derive(Debug)]
pub struct Configuration<V>
where
    V: Value + PartialOrd + Clone,
{
    pub false_fixed: Option<V>,
    pub true_fixed: Option<V>,
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
    signal_false: Option<signal::state_target_last::Signal<V>>,
    signal_true: Option<signal::state_target_last::Signal<V>>,
    signal_output: signal::state_source::Signal<bool>,
}
impl<V> Device<V>
where
    V: Value + PartialOrd + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        let false_fixed = configuration.false_fixed.is_some();
        let true_fixed = configuration.true_fixed.is_some();

        Self {
            configuration,
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_false: if !false_fixed {
                Some(signal::state_target_last::Signal::<V>::new())
            } else {
                None
            },
            signal_true: if !true_fixed {
                Some(signal::state_target_last::Signal::<V>::new())
            } else {
                None
            },
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn calculate(
        input: &V,
        false_: &V,
        true_: &V,
    ) -> Option<bool> {
        match false_.partial_cmp(true_) {
            Some(Ordering::Greater) => {
                if input <= true_ {
                    Some(true)
                } else if input >= false_ {
                    Some(false)
                } else {
                    None
                }
            }
            Some(Ordering::Less) => {
                if input <= false_ {
                    Some(false)
                } else if input >= true_ {
                    Some(true)
                } else {
                    None
                }
            }
            Some(Ordering::Equal) | None => None, // equal or unable to determine - leave unchanged
        }
    }
    fn calculate_optional(
        input: Option<&V>,
        false_: Option<&V>,
        true_: Option<&V>,
    ) -> Option<Option<bool>> {
        Some(Self::calculate(input?, false_?, true_?))
    }

    fn signals_targets_changed(&self) {
        let input = self.signal_input.take_last().value;
        let input = input.as_ref();

        let false_ = self
            .signal_false
            .as_ref()
            .and_then(|signal_false| signal_false.take_last().value);
        let false_ = match &self.configuration.false_fixed {
            Some(false_fixed) => Some(false_fixed),
            None => false_.as_ref(),
        };

        let true_ = self
            .signal_true
            .as_ref()
            .and_then(|signal_true| signal_true.take_last().value);
        let true_ = match &self.configuration.true_fixed {
            Some(true_fixed) => Some(true_fixed),
            None => true_.as_ref(),
        };

        let output = Self::calculate_optional(input, false_, true_);

        let output = match output {
            Some(Some(value)) => Some(value), // change to value
            None => None,                     // change to None
            Some(None) => return,             // unchanged
        };

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
        Cow::from(format!("soft/controller/bang_bang_a<{}>", type_name::<V>()))
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
    False,
    True,
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
            self.signal_false.as_ref().map(|signal_false| {
                (
                    SignalIdentifier::False, // line break
                    signal_false as &dyn signal::Base,
                )
            }),
            self.signal_true.as_ref().map(|signal_true| {
                (
                    SignalIdentifier::True, // line break
                    signal_true as &dyn signal::Base,
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
