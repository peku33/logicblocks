use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
};
use async_trait::async_trait;
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use std::{borrow::Cow, time::Duration};

#[derive(Debug)]
pub struct Breakpoint {
    pub expires: Duration, // after previous breakpoint
}

#[derive(Debug)]
pub struct Configuration {
    pub breakpoints: Vec<Breakpoint>,
}

// signal_output_started triggered in idle state, after signal goes from false to true
// signal_output_breakpoints[b].0 (aka released) triggered when signal goes from true to false before b breakpoint
// signal_output_breakpoints[b].1 (aka expired) triggered when b breakpoint is finished
// signal_output_finished triggered when signal goes from true to false after last breakpoint

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    input_waker: waker_stream::mpsc::SenderReceiver,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::state_target_last::Signal<bool>,
    signal_output_started: signal::event_source::Signal<()>,
    signal_output_breakpoints: Vec<(
        signal::event_source::Signal<()>, // released
        signal::event_source::Signal<()>, // expired
    )>,
    signal_output_finished: signal::event_source::Signal<()>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let breakpoints_count = configuration.breakpoints.len();

        Self {
            configuration,

            input_waker: waker_stream::mpsc::SenderReceiver::new(),

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::state_target_last::Signal::<bool>::new(),
            signal_output_started: signal::event_source::Signal::<()>::new(),
            signal_output_breakpoints: (0..breakpoints_count)
                .map(|_| {
                    (
                        signal::event_source::Signal::<()>::new(),
                        signal::event_source::Signal::<()>::new(),
                    )
                })
                .collect(),
            signal_output_finished: signal::event_source::Signal::<()>::new(),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let mut input_waker_receiver = self.input_waker.receiver();

        'outer: loop {
            // wait until signal goes to acquired state
            'wait_for_acquired: loop {
                // if signal is in active state - go to active part
                if self.signal_input.take_last().value.unwrap_or(false) {
                    break 'wait_for_acquired;
                }

                // wait for value change
                select! {
                    () = exit_flag => break 'outer,
                    () = input_waker_receiver.select_next_some() => {},
                }
            }
            if self.signal_output_started.push_one(()) {
                self.signal_sources_changed_waker.wake();
            }

            for (index, breakpoint) in self.configuration.breakpoints.iter().enumerate() {
                // create timer to wait for breakpoint time
                let breakpoint_timer = tokio::time::sleep(breakpoint.expires);
                pin_mut!(breakpoint_timer);
                let mut breakpoint_timer = breakpoint_timer.fuse();

                // tell whether client released the state or timeout expired
                let released = 'break_on_released: loop {
                    if !self.signal_input.take_last().value.unwrap_or(false) {
                        break 'break_on_released true;
                    }

                    // wait for value change
                    select! {
                        () = exit_flag => break 'outer,
                        () = input_waker_receiver.select_next_some() => {},
                        () = breakpoint_timer => break 'break_on_released false,
                    }
                };

                if released {
                    // trigger released signal
                    if self.signal_output_breakpoints[index].0.push_one(()) {
                        self.signal_sources_changed_waker.wake();
                    }

                    // button is released, we break the section
                    continue 'outer;
                } else {
                    // timeout expires, input still acquired

                    if self.signal_output_breakpoints[index].1.push_one(()) {
                        self.signal_sources_changed_waker.wake();
                    }
                }
            }

            // no breakpoint was hit, we are still acquired
            // wait for signal to be released and go to the beginning
            'wait_for_released: loop {
                if !self.signal_input.take_last().value.unwrap_or(false) {
                    break 'wait_for_released;
                }

                // wait for value change
                select! {
                    () = exit_flag => break 'outer,
                    () = input_waker_receiver.select_next_some() => {},
                }
            }
            if self.signal_output_finished.push_one(()) {
                self.signal_sources_changed_waker.wake();
            }
        }

        Exited
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/boolean_level_duration_a")
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_runnable(&self) -> Option<&dyn Runnable> {
        Some(self)
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
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        self.input_waker.wake();
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        std::iter::empty()
            .chain(std::array::IntoIter::new([
                &self.signal_input as &dyn signal::Base,          // 0
                &self.signal_output_started as &dyn signal::Base, // 1
            ]))
            .chain(
                self.signal_output_breakpoints
                    .iter()
                    .map(|(signal_released, signal_expired)| {
                        std::array::IntoIter::new([
                            signal_released as &dyn signal::Base, // 2 + 2b + 0
                            signal_expired as &dyn signal::Base,  // 2 + 2b + 1
                        ])
                    })
                    .flatten(),
            )
            .chain(std::array::IntoIter::new([
                &self.signal_output_finished as &dyn signal::Base, // 2 + 2B
            ]))
            .enumerate()
            .map(|(signal_id, signal)| (signal_id as signals::Id, signal))
            .collect()
    }
}
