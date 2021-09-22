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
use futures::{future::MaybeDone, pin_mut, select, StreamExt};
use maplit::hashmap;
use std::{borrow::Cow, time::Duration};

#[derive(Debug)]
pub struct Configuration {
    pub delay_raising: Duration,
    pub delay_falling: Duration,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    inner_waker: waker_stream::mpsc::SenderReceiver,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input: signal::state_target_last::Signal<bool>,
    signal_output: signal::state_source::Signal<bool>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            inner_waker: waker_stream::mpsc::SenderReceiver::new(),

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input: signal::state_target_last::Signal::<bool>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(None),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let mut inner_waker_receiver = self.inner_waker.receiver();

        loop {
            let state_next = self.signal_input.peek_last();

            let delay = match state_next {
                Some(true) => self.configuration.delay_raising,
                Some(false) => self.configuration.delay_falling,
                None => Duration::ZERO,
            };

            let delay_future = if delay >= Duration::ZERO {
                let future = tokio::time::sleep(delay);
                MaybeDone::Future(future)
            } else {
                MaybeDone::Done(())
            };
            pin_mut!(delay_future);

            select! {
                () = inner_waker_receiver.select_next_some() => continue,
                () = delay_future => {},
                () = exit_flag => break,
            }

            if self.signal_output.set_one(state_next) {
                self.signal_sources_changed_waker.wake();
            }

            select! {
                () = inner_waker_receiver.select_next_some() => {},
                () = exit_flag => break,
            }
        }

        Exited
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/slope_delay_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
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
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        if self.signal_input.take_pending().is_some() {
            self.inner_waker.wake();
        }
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_input as &dyn signal::Base,
            1 => &self.signal_output as &dyn signal::Base,
        }
    }
}
