use crate::{
    datatypes::real::Real,
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::StreamExt;
use maplit::hashmap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Configuration {
    // in ascending order, eg. y = 3x^2 + 2x + 1 will be [1, 2, 3]
    pub terms: Box<[Real]>,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_queued::Signal<Real>,
    signal_output: signal::state_source::Signal<Real>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_queued::Signal::<Real>::new(),
            signal_output: signal::state_source::Signal::<Real>::new(None),
        }
    }

    fn evaluate(
        &self,
        input: &Real,
    ) -> Real {
        let input = input.to_f64();

        let output = self
            .configuration
            .terms
            .iter()
            .map(|term| term.to_f64())
            .enumerate()
            .map(|(power, term)| term * input.powi(power as _))
            .sum();

        let output = Real::from_f64(output).unwrap();

        output
    }

    fn signals_targets_changed(&self) {
        let values = self
            .signal_input
            .take_pending()
            .into_iter()
            .map(|value| value.map(|value| self.evaluate(&value)))
            .collect::<Box<[Option<Real>]>>();

        if self.signal_output.set_many(values) {
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

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/math/polynomial_a")
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
    Input,
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
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
