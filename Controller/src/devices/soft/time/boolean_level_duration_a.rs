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

// last signals_output.expired will fire after last breakpoint is hit, meaning no more events will be triggered
// additionally signal_output_finished is similar, but will signal after button release

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    input_waker: waker_stream::mpsc::SenderReceiver,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::state_target_last::Signal<bool>,
    signals_output: Vec<(
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
            signals_output: (0..breakpoints_count)
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

            // check at which index signal was released
            // None - all breakpoints passed, signal never released
            let mut released = false;

            for (index, breakpoint) in self.configuration.breakpoints.iter().enumerate() {
                // create timer to wait for breakpoint time
                let breakpoint_timer = tokio::time::sleep(breakpoint.expires);
                pin_mut!(breakpoint_timer);
                let mut breakpoint_timer = breakpoint_timer.fuse();

                // tell whether client released the state or timeout expired
                let released_inner = 'break_on_released: loop {
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

                if released_inner {
                    // countdown was stopped by released state
                    // released state after loop, no need to wait
                    released = true;

                    // trigger released signal
                    if self.signals_output[index].0.push_one(()) {
                        self.signal_sources_changed_waker.wake();
                    }

                    // button is released, this is over
                    break;
                } else {
                    // timeout expires, input still acquired

                    if self.signals_output[index].1.push_one(()) {
                        self.signal_sources_changed_waker.wake();
                    }
                }
            }

            if !released {
                // wait for released state
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

                // run post release signal
                if self.signal_output_finished.push_one(()) {
                    self.signal_sources_changed_waker.wake();
                }
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
                &self.signal_input as &dyn signal::Base, // 0
            ]))
            .chain(
                self.signals_output
                    .iter()
                    .map(|(signal_released, signal_expired)| {
                        std::array::IntoIter::new([
                            signal_released as &dyn signal::Base, // 1 + 2b + 0
                            signal_expired as &dyn signal::Base,  // 1 + 2b + 1
                        ])
                    })
                    .flatten(),
            )
            .chain(std::array::IntoIter::new([
                &self.signal_output_finished as &dyn signal::Base, // 1 + 2B
            ]))
            .enumerate()
            .map(|(signal_id, signal)| (signal_id as signals::Id, signal))
            .collect()
    }
}
