use crate::{
    datatypes::ratio::Ratio,
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use maplit::hashmap;
use rand::{RngExt, rng};
use std::{
    borrow::Cow,
    ops::Rem,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub struct Configuration {
    /// full (on + off) cycle duration
    pub cycle_duration: Duration,
    /// jitter of cycle_duration, to prevent to prevent multiple instances of
    /// running in sync leave None to have it randomized
    pub cycle_phase_shift: Option<Ratio>,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_last::Signal<Ratio>,
    signal_output: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        assert!(configuration.cycle_duration > Duration::ZERO);

        Self {
            configuration,
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<Ratio>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let cycle_duration_seconds = self.configuration.cycle_duration.as_secs_f64();

        // randomized phase shift if user didn't provide it manually
        let cycle_phase_shift_ratio = self
            .configuration
            .cycle_phase_shift
            .unwrap_or_else(|| rng().random::<Ratio>())
            .to_f64();

        let signals_targets_changed_stream = self.signals_targets_changed_waker.stream();
        pin_mut!(signals_targets_changed_stream);

        enum CycleMode {
            Constant(Option<bool>),
            Variable(f64),
        }

        loop {
            // for None, zero and full we don't need any timer, instead we can propagate the
            // value to the output and wait until its changed
            let cycle_mode = match self.signal_input.take_last().value {
                None => CycleMode::Constant(None),
                Some(ratio) => {
                    if ratio == Ratio::zero() {
                        CycleMode::Constant(Some(false))
                    } else if ratio == Ratio::full() {
                        CycleMode::Constant(Some(true))
                    } else {
                        CycleMode::Variable(ratio.to_f64())
                    }
                }
            };

            match cycle_mode {
                CycleMode::Constant(output) => {
                    // in constant mode simply propagate the output and wait until value is changed
                    if self.signal_output.set_one(output) {
                        self.signals_sources_changed_waker.wake();
                    }

                    // wait for device exit or input change
                    select! {
                        () = signals_targets_changed_stream.select_next_some() => {},
                        () = exit_flag => break,
                    }
                }
                CycleMode::Variable(ratio_f64) => {
                    // system time, including frequency shift
                    let system_time_seconds = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();

                    // cycle phase 0.0 - 1.0, then added phase shift
                    let cycle_phase_ratio =
                        system_time_seconds.rem(cycle_duration_seconds) / cycle_duration_seconds;
                    let cycle_phase_ratio = (cycle_phase_ratio + cycle_phase_shift_ratio).rem(1.0);

                    // on from 0.0 -> ratio_f64
                    // off from ratio_f64 -> 1.0

                    // output value, ratio to nearest change
                    let (output, cycle_output_remaining_ratio) = if cycle_phase_ratio <= ratio_f64 {
                        (true, ratio_f64 - cycle_phase_ratio)
                    } else {
                        (false, 1.0 - cycle_phase_ratio)
                    };

                    // set output
                    if self.signal_output.set_one(Some(output)) {
                        self.signals_sources_changed_waker.wake();
                    }

                    // calculate time until next change
                    let cycle_output_remaining = self
                        .configuration
                        .cycle_duration
                        .mul_f64(cycle_output_remaining_ratio);

                    // to prevent loops around time of change we add 10 msec of extra delay
                    let cycle_output_remaining = cycle_output_remaining + Duration::from_millis(10);

                    // wait for input change / state change, exit signal
                    select! {
                        () = signals_targets_changed_stream.select_next_some() => {},
                        () = tokio::time::sleep(cycle_output_remaining).fuse() => {},
                        () = exit_flag => break,
                    }
                }
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/pwm_slow_a")
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
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
