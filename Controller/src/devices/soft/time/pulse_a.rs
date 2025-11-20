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
use itertools::chain;
use std::{borrow::Cow, iter, time::Duration};

#[derive(Debug)]
pub struct Configuration {
    pub duration_fixed: Option<Duration>,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_duration: Option<signal::state_target_last::Signal<Duration>>,
    signal_trigger: signal::event_target_last::Signal<()>,
    signal_cancel: signal::event_target_last::Signal<()>,
    signal_output: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let duration_fixed = configuration.duration_fixed.is_some();

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_duration: if !duration_fixed {
                Some(signal::state_target_last::Signal::<Duration>::new())
            } else {
                None
            },
            signal_trigger: signal::event_target_last::Signal::<()>::new(),
            signal_cancel: signal::event_target_last::Signal::<()>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(Some(false)),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let signals_targets_changed_stream = self.signals_targets_changed_waker.stream();
        pin_mut!(signals_targets_changed_stream);

        'outer: loop {
            // off-state
            // set signal
            if self.signal_output.set_one(Some(false)) {
                self.signals_sources_changed_waker.wake();
            }

            // on trigger - break to enter on-state loop
            // on cancel - continue waiting in off-state
            // on exit - exit
            loop {
                select! {
                    () = signals_targets_changed_stream.select_next_some() => {
                        #[allow(clippy::single_match)]
                        match (
                            self.signal_trigger.take_pending().is_some(),
                            self.signal_cancel.take_pending().is_some()
                        ) {
                            (true, false) => break, // trigger detected, go to on-state
                            _ => {}, // either no trigger or cancel or both - stay in off-state
                        }
                    },
                    () = exit_flag => break 'outer,
                }
            }

            // on-state
            'inner: loop {
                // establish duration
                let duration = self
                    .signal_duration
                    .as_ref()
                    .and_then(|signal_duration| signal_duration.take_last().value);
                let duration = match &self.configuration.duration_fixed {
                    Some(duration_fixed) => Some(duration_fixed),
                    None => duration.as_ref(),
                };

                // skip if duration not set (eg. not fixed and not connected)
                let duration = match duration {
                    Some(duration) => duration,
                    None => break,
                };

                // set signal
                if self.signal_output.set_one(Some(true)) {
                    self.signals_sources_changed_waker.wake();
                }

                // set up timer
                let timer = tokio::time::sleep(*duration).fuse();
                pin_mut!(timer);

                // on trigger - restart on-loop with fresh duration
                // on cancel or timer - exit inner loop to go to off-state
                // on exit - exit
                loop {
                    select! {
                        () = signals_targets_changed_stream.select_next_some() => {
                            match (
                                self.signal_trigger.take_pending().is_some(),
                                self.signal_cancel.take_pending().is_some()
                            ) {
                                (true, false) => break, // re-triggered - reset on-state
                                (_, true) => break 'inner, // cancelled - go to off-state
                                _ => {}, // nothing interesting, keep waiting
                            }
                        },
                        () = timer => break 'inner, // duration elapsed - go to off state
                        () = exit_flag => break 'outer,
                    }
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
    Duration,
    Trigger,
    Cancel,
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
        chain!(
            self.signal_duration.as_ref().map(|signal_duration| {
                (
                    SignalIdentifier::Duration, // line break
                    signal_duration as &dyn signal::Base,
                )
            }),
            iter::once((
                SignalIdentifier::Trigger,
                &self.signal_trigger as &dyn signal::Base,
            )),
            iter::once((
                SignalIdentifier::Cancel,
                &self.signal_cancel as &dyn signal::Base,
            )),
            iter::once((
                SignalIdentifier::Output,
                &self.signal_output as &dyn signal::Base,
            )),
        )
        .collect::<signals::ByIdentifier<_>>()
    }
}
