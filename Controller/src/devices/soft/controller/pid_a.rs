use crate::{
    datatypes::{duration::Duration, real::Real},
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::{
    future::{Fuse, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use maplit::hashmap;
use parking_lot::RwLock;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Configuration {
    pub tick_duration: Duration,

    pub pid: controller::Configuration,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    pid: RwLock<Option<controller::PID>>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_setpoint: signal::state_target_last::Signal<Real>,
    signal_measurement: signal::state_target_last::Signal<Real>,
    signal_output: signal::state_source::Signal<Real>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let pid = RwLock::<Option<controller::PID>>::new(None);

        Self {
            configuration,
            pid,
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_setpoint: signal::state_target_last::Signal::<Real>::new(),
            signal_measurement: signal::state_target_last::Signal::<Real>::new(),
            signal_output: signal::state_source::Signal::<Real>::new(None),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let signals_targets_changed_stream = self.signals_targets_changed_waker.stream();
        pin_mut!(signals_targets_changed_stream);

        let tick_next = Fuse::<tokio::time::Sleep>::terminated();
        pin_mut!(tick_next);

        loop {
            let elapsed = select! {
                () = signals_targets_changed_stream.select_next_some() => false,
                _ = &mut tick_next => true,
                () = exit_flag => break,
            };

            let mut pid = self.pid.write();
            let pid = &mut *pid;

            let setpoint = self.signal_setpoint.take_last().value;
            let measurement = self.signal_measurement.take_last().value;

            match setpoint.zip(measurement) {
                Some((setpoint, measurement)) => {
                    // input and setpoint are set
                    // either initialize the pid or tick if was initialized
                    match pid {
                        Some(pid) => {
                            // either input was change or timer has elapsed
                            // we are interested only only in timer elapsed for next tick
                            if !elapsed {
                                continue;
                            }

                            let output = pid.tick(setpoint, measurement);

                            if self.signal_output.set_one(Some(output)) {
                                self.signals_sources_changed_waker.wake();
                            }
                        }
                        None => {
                            // pid wasn't initialized, so tick_next also wasn't and output was None
                            // so we are here because input changed from none to not-none
                            *pid = Some(controller::PID::new(self.configuration.pid, measurement));
                            debug_assert!(!elapsed);
                        }
                    }

                    // either timer has elapsed or it wasn't initialized
                    tick_next
                        .set(tokio::time::sleep(self.configuration.tick_duration.to_std()).fuse());
                }
                None => {
                    // input was changed to (or is) none, remove pid, reset signals and timer
                    *pid = None;

                    if self.signal_output.set_one(None) {
                        self.signals_sources_changed_waker.wake();
                    }

                    tick_next.set(Fuse::terminated());
                }
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/controller/pid_a")
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
    Setpoint,
    Measurement,
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
            SignalIdentifier::Setpoint => &self.signal_setpoint as &dyn signal::Base,
            SignalIdentifier::Measurement => &self.signal_measurement as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}

pub mod controller {
    use crate::datatypes::{range::Range, real::Real};
    use derive_more::Debug;

    #[derive(Clone, Copy, Debug)]
    pub struct Configuration {
        pub p: Real,
        pub i: Real,
        pub d: Real,

        pub p_limit: Option<Range<Real>>,
        pub i_limit: Option<Range<Real>>,
        pub d_limit: Option<Range<Real>>,

        pub output_limit: Option<Range<Real>>,
    }

    #[derive(Debug)]
    pub struct PID {
        configuration: Configuration,

        measurement_previous: Real,
        i: Real,
    }
    impl PID {
        pub fn new(
            configuration: Configuration,
            measurement: Real,
        ) -> Self {
            let i = Real::zero();

            Self {
                configuration,
                measurement_previous: measurement,
                i,
            }
        }

        pub fn tick(
            &mut self,
            setpoint: Real,
            measurement: Real,
        ) -> Real {
            let error = setpoint - measurement;

            // p
            let p = error * self.configuration.p;
            let p = match self.configuration.p_limit {
                Some(p_limit) => p_limit.clamp_to(p),
                None => p,
            };

            // i
            let i = self.i + error * self.configuration.i;
            let i = match self.configuration.i_limit {
                Some(i_limit) => i_limit.clamp_to(i),
                None => i,
            };

            // d
            let d = (measurement - self.measurement_previous) * self.configuration.d;
            let d = match self.configuration.d_limit {
                Some(d_limit) => d_limit.clamp_to(d),
                None => d,
            };

            // output
            let output = p + i + d;
            let output = match self.configuration.output_limit {
                Some(output_limit) => output_limit.clamp_to(output),
                None => output,
            };

            // update self
            self.measurement_previous = measurement;
            self.i = i;

            output
        }
    }
}
