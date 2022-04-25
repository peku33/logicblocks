use crate::{
    datatypes::ratio::Ratio,
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use std::{borrow::Cow, iter};

#[derive(Debug)]
pub struct Configuration {
    pub inputs_count: usize,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signals_input: Vec<signal::state_target_last::Signal<bool>>,
    signal_output: signal::state_source::Signal<Ratio>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let inputs_count = configuration.inputs_count;

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signals_input: (0..inputs_count)
                .map(|_input_id| signal::state_target_last::Signal::<bool>::new())
                .collect::<Vec<_>>(),
            signal_output: signal::state_source::Signal::<Ratio>::new(None),
        }
    }

    fn signals_targets_changed(&self) {
        let inputs_values = self
            .signals_input
            .iter()
            .map(|signal_input| signal_input.take_last())
            .collect::<Vec<_>>();

        // if no signal is pending, don't recalculate
        if !inputs_values.iter().any(|value| value.pending) {
            return;
        }

        let counts_known = inputs_values
            .iter()
            .filter(|last| last.value.is_some())
            .count();
        let counts_one = inputs_values
            .iter()
            .filter(|last| last.value.contains(&true))
            .count();

        let ratio = (counts_one as f64) / (counts_known as f64);
        let ratio: Option<Ratio> = if ratio.is_finite() {
            Some(Ratio::from_f64(ratio).unwrap())
        } else {
            // eg. division by zero
            None
        };

        if self.signal_output.set_one(ratio) {
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

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/encoders_decoders/boolean_to_ratio_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
}

#[async_trait]
impl Runnable for Device {
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
impl signals::Device for Device {
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
                self.signals_input
                    .iter()
                    .enumerate()
                    .map(|(input_index, input_signal)| {
                        (
                            SignalIdentifier::Input(input_index),
                            input_signal as &dyn signal::Base,
                        )
                    }),
            )
            .chain([(
                SignalIdentifier::Output,
                &self.signal_output as &dyn signal::Base,
            )])
            .collect()
    }
}