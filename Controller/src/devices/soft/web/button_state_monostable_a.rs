use crate::{
    devices, signals,
    signals::signal,
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
    web::{self, uri_cursor},
};
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    pin_mut, select,
};
use maplit::hashmap;
use std::{borrow::Cow, time::Duration};
use tokio::sync::watch;

#[derive(Debug)]
pub struct Device {
    value_beat_sender: watch::Sender<bool>,
    value_beat_receiver: watch::Receiver<bool>,

    gui_summary_waker: waker_stream::mpmc::Sender,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_output: signal::state_source::Signal<bool>,
}
impl Device {
    const VALUE_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn new() -> Self {
        let (value_beat_sender, value_beat_receiver) = watch::channel(false);

        Self {
            value_beat_sender,
            value_beat_receiver,

            gui_summary_waker: waker_stream::mpmc::Sender::new(),

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_output: signal::state_source::Signal::<bool>::new(Some(false)),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let mut value_beat_receiver = self.value_beat_receiver.clone();

        'outer: loop {
            // wait for signal to go up
            'inner_wait_for_up: loop {
                if *value_beat_receiver.borrow_and_update() {
                    break 'inner_wait_for_up;
                }

                select! {
                    () = exit_flag => break 'outer,
                    result = value_beat_receiver.changed().fuse() => {
                        result.unwrap();
                        continue 'inner_wait_for_up;
                    },
                }
            }
            if self.signal_output.set_one(Some(true)) {
                self.signal_sources_changed_waker.wake();
            }

            // wait for signal to go down or timeout expires
            'inner_wait_for_down: loop {
                if !*value_beat_receiver.borrow_and_update() {
                    break 'inner_wait_for_down;
                }

                let timeout = tokio::time::sleep(Self::VALUE_TIMEOUT);
                pin_mut!(timeout);
                let mut timeout = timeout.fuse();

                select! {
                    () = exit_flag => break 'outer,
                    result = value_beat_receiver.changed().fuse() => {
                        result.unwrap();
                        continue 'inner_wait_for_down;
                    },
                    () = timeout => {},
                }

                // timeout expired
                self.value_beat_sender.send(false).unwrap();
                self.gui_summary_waker.wake();

                break 'inner_wait_for_down;
            }
            if self.signal_output.set_one(Some(false)) {
                self.signal_sources_changed_waker.wake();
            }
        }
        Exited
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/web/button_state_monostable_a")
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_runnable(&self) -> Option<&dyn Runnable> {
        Some(self)
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
        Some(self)
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        Some(self)
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        // no signal targets
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_output as &dyn signal::Base,
        }
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
impl devices::GuiSummaryProvider for Device {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        let gui_summary = *self.value_beat_receiver.borrow();
        let gui_summary = Box::new(gui_summary);
        gui_summary
    }

    fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
impl uri_cursor::Handler for Device {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::POST => {
                    let value = match request.body_parse_json::<bool>() {
                        Ok(value) => value,
                        Err(error) => {
                            return async move { web::Response::error_400_from_error(error) }
                                .boxed()
                        }
                    };

                    self.value_beat_sender.send(value).unwrap();
                    self.gui_summary_waker.wake();

                    async move { web::Response::ok_empty() }.boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
