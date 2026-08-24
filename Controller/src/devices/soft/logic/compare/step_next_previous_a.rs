use crate::{
    devices,
    signals::{self, signal, types::state::Value as StateValue},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow, collections::BTreeSet, ops::Bound};

#[derive(Clone, Copy, Debug)]
enum Direction {
    Next,
    Previous,
}

#[derive(Debug)]
pub struct Configuration<V>
where
    V: StateValue + Ord + Clone,
{
    /// Breakpoints to step through, kept sorted and unique by the set. Must
    /// not be empty.
    pub values: BTreeSet<V>,

    /// Behavior when stepping past an end:
    /// - `true` - wrap around (last -> first, first -> last).
    /// - `false` - no value on the given output,
    pub wrap: bool,

    /// Behavior when no input value is present yet (input never set /
    /// disconnected):
    /// - `true` - `next` yields the first breakpoint, `previous` the last.
    /// - `false` - no value on either output,
    pub missing_restart: bool,
}

#[derive(Debug)]
pub struct Device<V>
where
    V: StateValue + Ord + Clone,
{
    configuration: Configuration<V>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_last::Signal<V>,
    signal_previous: signal::state_source::Signal<V>,
    signal_next: signal::state_source::Signal<V>,
}
impl<V> Device<V>
where
    V: StateValue + Ord + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        assert!(!configuration.values.is_empty());

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_previous: signal::state_source::Signal::<V>::new(None),
            signal_next: signal::state_source::Signal::<V>::new(None),
        }
    }

    /// Resolves the breakpoint in `direction`, given the `input` value (`None`
    /// if the input is not set).
    fn calculate<'v>(
        configuration: &'v Configuration<V>,
        input: Option<&'v V>,
        direction: Direction,
    ) -> Option<&'v V> {
        let output = match input {
            Some(input) => {
                // input value is provided, so we can find the closes
                // breakpoint. the `values` must not be empty, so unwrap never
                // fails
                match direction {
                    Direction::Next => {
                        match configuration
                            .values
                            .range((Bound::Excluded(input), Bound::Unbounded))
                            .next()
                        {
                            Some(value) => {
                                // next value found, return it
                                Some(value)
                            }
                            None => {
                                // no next value, either wrap or keep on last
                                // value depending on configuration
                                if configuration.wrap {
                                    Some(configuration.values.first().unwrap())
                                } else {
                                    // the last value could be before input, don't do anything
                                    None
                                }
                            }
                        }
                    }
                    Direction::Previous => {
                        match configuration
                            .values
                            .range((Bound::Unbounded, Bound::Excluded(input)))
                            .next_back()
                        {
                            Some(value) => {
                                // previous value found, return it
                                Some(value)
                            }
                            None => {
                                // no previous value, either wrap or keep on
                                // last value depending on configuration
                                if configuration.wrap {
                                    Some(configuration.values.last().unwrap())
                                } else {
                                    None
                                }
                            }
                        }
                    }
                }
            }
            None => {
                // there is no input value, so restart or do nothing, depending on configuration
                if configuration.missing_restart {
                    let output = match direction {
                        Direction::Next => configuration.values.first().unwrap(),
                        Direction::Previous => configuration.values.last().unwrap(),
                    };

                    Some(output)
                } else {
                    None
                }
            }
        };

        output
    }

    fn signals_targets_changed(&self) {
        let mut signals_sources_changed = false;

        let input = self.signal_input.take_last().value;

        let previous = Self::calculate(&self.configuration, input.as_ref(), Direction::Previous);
        let next = Self::calculate(&self.configuration, input.as_ref(), Direction::Next);

        signals_sources_changed |= self.signal_previous.set_one(previous.cloned());
        signals_sources_changed |= self.signal_next.set_one(next.cloned());

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
    V: StateValue + Ord + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!(
            "soft/logic/compare/step_next_previous_a<{}>",
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
    V: StateValue + Ord + Clone,
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
    Previous,
    Next,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: StateValue + Ord + Clone,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Previous => &self.signal_previous as &dyn signal::Base,
            SignalIdentifier::Next => &self.signal_next as &dyn signal::Base,
        }
    }
}
