use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use std::{borrow::Cow, iter};

#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub inputs_count: usize,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_inputs: Box<[signal::state_target_last::Signal<bool>]>,
    signal_output: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_inputs: (0..configuration.inputs_count)
                .map(|_| signal::state_target_last::Signal::<bool>::new())
                .collect::<Box<[_]>>(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    fn signals_targets_changed(&self) {
        let mut any_set = false;
        let mut any_true = false;

        for signal_input in self.signal_inputs.iter() {
            let value = signal_input.take_last().value;

            if let Some(value) = value {
                any_set = true;

                if value {
                    any_true = true;
                }
            }
        }

        let value = match (any_set, any_true) {
            (true, value) => Some(value),
            (false, _) => None,
        };

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
            .for_each(async move |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/boolean/gate/or_a")
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
