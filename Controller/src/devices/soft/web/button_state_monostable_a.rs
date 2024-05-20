use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
    web::{self, uri_cursor},
};
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    pin_mut, select,
};
use maplit::hashmap;
use serde::Serialize;
use std::{borrow::Cow, time::Duration};
use tokio::sync::watch;

#[derive(Debug)]
pub struct Device {
    value_beat_sender: watch::Sender<bool>,
    value_beat_receiver: watch::Receiver<bool>,

    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_output: signal::state_source::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    const VALUE_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn new() -> Self {
        let (value_beat_sender, value_beat_receiver) = watch::channel(false);

        Self {
            value_beat_sender,
            value_beat_receiver,

            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_output: signal::state_source::Signal::<bool>::new(Some(false)),

            gui_summary_waker: devices::gui_summary::Waker::new(),
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
                self.signals_sources_changed_waker.wake();
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
                self.signals_sources_changed_waker.wake();
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/web/button_state_monostable_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
    fn as_gui_summary_device_base(&self) -> Option<&dyn devices::gui_summary::DeviceBase> {
        Some(self)
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl signals::Device for Device {
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        None
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
        hashmap! {
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct GuiSummary {
    value: bool,
}
impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let value = *self.value_beat_receiver.borrow();
        Self::Value { value }
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
                            return async { web::Response::error_400_from_error(error) }.boxed()
                        }
                    };

                    self.value_beat_sender.send(value).unwrap();
                    self.gui_summary_waker.wake();

                    async { web::Response::ok_empty() }.boxed()
                }
                _ => async { web::Response::error_405() }.boxed(),
            },
            _ => async { web::Response::error_404() }.boxed(),
        }
    }
}
