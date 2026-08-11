use crate::{
    devices,
    signals::{
        self, signal,
        types::{event::Value as EventValue, state::Value as StateValue},
    },
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use itertools::{chain, zip_eq};
use std::{any::type_name, borrow::Cow, iter};

#[derive(Debug)]
pub struct Configuration {
    /// number of input + trigger pairs
    pub inputs_count: usize,
}

/// State<V> signals are provided on input targets
/// when Event<()> hits matching trigger target
/// current input value is emitted as Event<V> from output source
#[derive(Debug)]
pub struct Device<V>
where
    V: EventValue + StateValue + Clone,
{
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_inputs: Box<[signal::state_target_last::Signal<V>]>,
    signal_triggers: Box<[signal::event_target_last::Signal<()>]>,
    signal_output: signal::event_source::Signal<V>,
}
impl<V> Device<V>
where
    V: EventValue + StateValue + Clone,
{
    pub fn new(configuration: Configuration) -> Self {
        let inputs_count = configuration.inputs_count;

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_inputs: (0..inputs_count)
                .map(|_input_index| signal::state_target_last::Signal::<V>::new())
                .collect::<Box<[_]>>(),
            signal_triggers: (0..inputs_count)
                .map(|_input_index| signal::event_target_last::Signal::<()>::new())
                .collect::<Box<[_]>>(),
            signal_output: signal::event_source::Signal::<V>::new(),
        }
    }

    fn signals_targets_changed(&self) {
        // iterated to the end, so all triggers are drained
        let outputs = zip_eq(self.signal_inputs.iter(), self.signal_triggers.iter())
            .filter_map(|(signal_input, signal_trigger)| {
                match (
                    signal_input.take_last().value,
                    signal_trigger.take_pending(),
                ) {
                    (_, None) => None,                      // trigger not triggered
                    (None, _) => None,                      // triggered, but input not set
                    (Some(input), Some(())) => Some(input), // triggered and input set
                }
            })
            .collect::<Box<[_]>>();

        if self.signal_output.push_many(outputs) {
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
    V: EventValue + StateValue + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/value/sample_a<{}>", type_name::<V>()))
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
    V: EventValue + StateValue + Clone,
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
    Input(usize),
    Trigger(usize),
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: EventValue + StateValue + Clone,
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
            self.signal_inputs
                .iter()
                .enumerate()
                .map(|(input_index, signal_input)| {
                    (
                        SignalIdentifier::Input(input_index),
                        signal_input as &dyn signal::Base,
                    )
                }),
            self.signal_triggers
                .iter()
                .enumerate()
                .map(|(input_index, signal_trigger)| {
                    (
                        SignalIdentifier::Trigger(input_index),
                        signal_trigger as &dyn signal::Base,
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
