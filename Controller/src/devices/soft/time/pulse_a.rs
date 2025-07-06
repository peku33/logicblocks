use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::{FutureExt, pin_mut, select, stream::StreamExt};
use maplit::hashmap;
use std::{borrow::Cow, time::Duration};

#[derive(Debug)]
pub struct Configuration {
    pub duration: Duration,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::event_target_last::Signal<()>,
    signal_output: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::event_target_last::Signal::<()>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let signal_input_changed_stream = self
            .signals_targets_changed_waker
            .stream()
            .filter(|_| async { self.signal_input.take_pending().is_some() });
        pin_mut!(signal_input_changed_stream);

        'outer: loop {
            // start in off state
            if self.signal_output.set_one(Some(false)) {
                self.signals_sources_changed_waker.wake();
            }

            // wait for event (or exit)
            select! {
                () = signal_input_changed_stream.select_next_some() => {},
                () = exit_flag => break,
            }

            // event detected
            loop {
                // enable
                if self.signal_output.set_one(Some(true)) {
                    self.signals_sources_changed_waker.wake();
                }

                // wait until timeout expires, restart if new event is detected
                select! {
                    () = signal_input_changed_stream.select_next_some() => continue,
                    () = tokio::time::sleep(self.configuration.duration).fuse() => break,
                    () = exit_flag => break 'outer,
                }
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/pulse_a")
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
