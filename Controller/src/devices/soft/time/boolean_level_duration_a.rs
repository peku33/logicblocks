use crate::{
    devices,
    signals::{self, signal, utils::state_target_queued_stream::StateTargetQueuedStream},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use std::{borrow::Cow, iter, time::Duration};

#[derive(Debug)]
pub struct Breakpoint {
    pub expires: Duration, // after previous breakpoint
}

#[derive(Debug)]
pub struct Configuration {
    pub breakpoints: Box<[Breakpoint]>,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_queued::Signal<bool>,
    signal_started: signal::event_source::Signal<()>,
    signal_breakpoints: Box<
        [(
            signal::event_source::Signal<()>, // released
            signal::event_source::Signal<()>, // expired
        )],
    >,
    signal_finished: signal::event_source::Signal<()>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let breakpoints_count = configuration.breakpoints.len();

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_queued::Signal::<bool>::new(),
            signal_started: signal::event_source::Signal::<()>::new(),
            signal_breakpoints: (0..breakpoints_count)
                .map(|_breakpoint_index| {
                    (
                        signal::event_source::Signal::<()>::new(),
                        signal::event_source::Signal::<()>::new(),
                    )
                })
                .collect::<Box<[_]>>(),
            signal_finished: signal::event_source::Signal::<()>::new(),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let signal_input_stream_filtered =
            StateTargetQueuedStream::new(&self.signals_targets_changed_waker, &self.signal_input)
                .filter_map(async |input| input);
        pin_mut!(signal_input_stream_filtered);

        'outer: loop {
            // wait until signal goes into active state
            'wait_for_active: loop {
                select! {
                    () = exit_flag => break 'outer,
                    signal_input_value = signal_input_stream_filtered.select_next_some() => {
                        // if signal is in active state - exit the waiting loop
                        // if not (this should never happen) - continue waiting
                        if signal_input_value {
                            break 'wait_for_active;
                        }
                    },
                }
            }
            if self.signal_started.push_one(()) {
                self.signals_sources_changed_waker.wake();
            }

            for (index, breakpoint) in self.configuration.breakpoints.iter().enumerate() {
                // create timer to wait for breakpoint time
                let breakpoint_timer = tokio::time::sleep(breakpoint.expires);
                pin_mut!(breakpoint_timer);
                let mut breakpoint_timer = breakpoint_timer.fuse();

                // tell whether client released the state or timeout expired
                let released = 'break_on_released: loop {
                    select! {
                        () = exit_flag => break 'outer,
                        signal_input_value = signal_input_stream_filtered.select_next_some() => {
                            // if client deasserted the input - exit the loop, "breaking here"
                            // if not (this should never happen) - continue waiting
                            if !signal_input_value {
                                break 'break_on_released true;
                            }
                        },
                        () = breakpoint_timer => break 'break_on_released false,
                    }
                };

                if released {
                    // trigger released signal
                    if self.signal_breakpoints[index].0.push_one(()) {
                        self.signals_sources_changed_waker.wake();
                    }

                    // button is released, we break the section
                    continue 'outer;
                } else {
                    // timeout expires, input still acquired

                    if self.signal_breakpoints[index].1.push_one(()) {
                        self.signals_sources_changed_waker.wake();
                    }
                }
            }

            // no breakpoint was hit, we are still acquired
            // wait for signal to be released and go to the beginning
            'wait_for_released: loop {
                // wait for value change
                select! {
                    () = exit_flag => break 'outer,
                    signal_input_value = signal_input_stream_filtered.select_next_some() => {
                        // if client released the button - exit the loop
                        // if not (this should never happen) - continue waiting
                        if !signal_input_value {
                            break 'wait_for_released;
                        }
                    },
                }
            }
            if self.signal_finished.push_one(()) {
                self.signals_sources_changed_waker.wake();
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/boolean_level_duration_a")
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
    Started,         // triggered in idle state, after signal goes from false to true
    Released(usize), // triggered when signal goes from true to false before .0 breakpoint
    Expired(usize),  // triggered when .0 breakpoint is finished
    Finished,        // triggered when signal goes from true to false after last breakpoint
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
            .chain([
                (
                    SignalIdentifier::Input,
                    &self.signal_input as &dyn signal::Base,
                ),
                (
                    SignalIdentifier::Started,
                    &self.signal_started as &dyn signal::Base,
                ),
            ])
            .chain(self.signal_breakpoints.iter().enumerate().flat_map(
                |(breakpoint_index, (signal_released, signal_expired))| {
                    [
                        (
                            SignalIdentifier::Released(breakpoint_index),
                            signal_released as &dyn signal::Base,
                        ),
                        (
                            SignalIdentifier::Expired(breakpoint_index),
                            signal_expired as &dyn signal::Base,
                        ),
                    ]
                },
            ))
            .chain([(
                SignalIdentifier::Finished,
                &self.signal_finished as &dyn signal::Base,
            )])
            .collect::<signals::ByIdentifier<_>>()
    }
}
