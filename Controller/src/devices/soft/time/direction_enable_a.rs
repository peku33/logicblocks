use crate::{
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
use std::{borrow::Cow, time::Duration};

#[derive(Debug)]
pub struct Configuration {
    // delay between enabling the direction signal and enabling the drive signal
    pub direction_pre_delay: Duration,

    // time required for enable signal to perform operation
    pub enable_duration: Duration,

    // delay between disabling the enable signal and disabling the direction signal
    pub direction_post_delay: Duration,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_setpoint: signal::state_target_last::Signal<bool>,
    signal_direction: signal::state_source::Signal<bool>,
    signal_enable: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_setpoint: signal::state_target_last::Signal::<bool>::new(),
            signal_direction: signal::state_source::Signal::<bool>::new(Some(false)),
            signal_enable: signal::state_source::Signal::<bool>::new(Some(false)),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let signals_targets_changed_stream = self.signals_targets_changed_waker.stream();
        pin_mut!(signals_targets_changed_stream);

        loop {
            // wait for signal change
            select! {
                () = signals_targets_changed_stream.select_next_some() => {},
                () = exit_flag => break,
            }

            // set the direction with unknown signal fallback as false
            let setpoint = self.signal_setpoint.take_last().value;
            if self
                .signal_direction
                .set_one(Some(setpoint.unwrap_or(false)))
            {
                self.signals_sources_changed_waker.wake();
            }
            // if direction is unspecified - don't run the action
            if setpoint.is_none() {
                continue;
            }

            // wait after setting the direction, but before enabling the drive
            // TODO: probably we don't have to wait if signal was on false for long
            select! {
                () = tokio::time::sleep(self.configuration.direction_pre_delay).fuse() => {},
                () = exit_flag => break,
            }

            // now enable the drive
            if self.signal_enable.set_one(Some(true)) {
                self.signals_sources_changed_waker.wake();
            }

            // wait for the drive enable time
            select! {
                () = tokio::time::sleep(self.configuration.enable_duration).fuse() => {},
                () = exit_flag => break,
            }

            // disable the drive
            if self.signal_enable.set_one(Some(false)) {
                self.signals_sources_changed_waker.wake();
            }

            // wait after stopping the drive, before disabling the direction
            select! {
                () = tokio::time::sleep(self.configuration.direction_post_delay).fuse() => {},
                () = exit_flag => break,
            }

            // disable the direction
            if self.signal_direction.set_one(Some(false)) {
                self.signals_sources_changed_waker.wake();
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/direction_enable_a")
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
    Direction,
    Enable,
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
            SignalIdentifier::Direction => &self.signal_direction as &dyn signal::Base,
            SignalIdentifier::Enable => &self.signal_enable as &dyn signal::Base,
        }
    }
}
