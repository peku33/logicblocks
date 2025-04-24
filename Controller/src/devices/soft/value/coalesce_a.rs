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
pub struct Configuration {
    pub inputs_count: usize,
}

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + Clone,
{
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_inputs: Box<[signal::state_target_last::Signal<V>]>,
    signal_output: signal::state_source::Signal<V>,
}
impl<V> Device<V>
where
    V: Value + Clone,
{
    pub fn new(configuration: Configuration) -> Self {
        let inputs_count = configuration.inputs_count;

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_inputs: (0..inputs_count)
                .map(|_input_id| signal::state_target_last::Signal::<V>::new())
                .collect::<Box<[_]>>(),
            signal_output: signal::state_source::Signal::<V>::new(None),
        }
    }

    fn signals_targets_changed(&self) {
        let inputs_values = self
            .signal_inputs
            .iter()
            .map(|signal_input| signal_input.take_last())
            .collect::<Box<[_]>>();

        // get first non-None value
        let value = inputs_values
            .into_iter()
            .find_map(|value| value.value);

        if self.signal_output.set_one(value) {
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
    V: Value + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/value/coalesce_a<{}>", type_name::<V>()))
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
    V: Value + Clone,
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
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: Value + Clone,
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
            .chain(
                self.signal_inputs
                    .iter()
                    .enumerate()
                    .map(|(input_index, signal_input)| {
                        (
                            SignalIdentifier::Input(input_index),
                            signal_input as &dyn signal::Base,
                        )
                    }),
            )
            .chain([(
                SignalIdentifier::Output,
                &self.signal_output as &dyn signal::Base,
            )])
            .collect::<signals::ByIdentifier<_>>()
    }
}
