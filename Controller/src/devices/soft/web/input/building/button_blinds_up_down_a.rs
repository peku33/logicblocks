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
    future::{BoxFuture, Fuse, FutureExt},
    pin_mut, select,
};
use maplit::hashmap;
use serde::Serialize;
use std::{borrow::Cow, time::Duration};
use tokio::sync::watch;

// TODO: add transition delay

#[derive(Debug)]
pub struct Device {
    value_beat_sender: watch::Sender<Option<bool>>,
    value_beat_receiver: watch::Receiver<Option<bool>>,

    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_down: signal::state_source::Signal<bool>,
    signal_up: signal::state_source::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    const VALUE_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn new() -> Self {
        let (value_beat_sender, value_beat_receiver) = watch::channel(None);

        Self {
            value_beat_sender,
            value_beat_receiver,

            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_down: signal::state_source::Signal::<bool>::new(Some(false)),
            signal_up: signal::state_source::Signal::<bool>::new(Some(false)),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let mut value_beat_receiver = self.value_beat_receiver.clone();

        let tick_next = Fuse::<tokio::time::Sleep>::terminated();
        pin_mut!(tick_next);

        'outer: loop {
            let direction = loop {
                select! {
                    result = value_beat_receiver.changed().fuse() => {
                        result.unwrap();
                        break *value_beat_receiver.borrow_and_update();
                    }
                    () = &mut tick_next => {
                        // this will be picked by value_beat_receiver branch
                        self.value_beat_sender.send(None).unwrap();
                    }
                    () = exit_flag => break 'outer,
                }
            };

            let (value_down, value_up) = match direction {
                Some(false) => (true, false),
                Some(true) => (false, true),
                None => (false, false),
            };
            let mut signals_sources_changed = false;
            signals_sources_changed |= self.signal_down.set_one(Some(value_down));
            signals_sources_changed |= self.signal_up.set_one(Some(value_up));
            if signals_sources_changed {
                self.signals_sources_changed_waker.wake();
            }

            self.gui_summary_waker.wake();

            // if we are currently in non-null state - set the timer to timeout
            // it will be either bumped by web request, disabled by web request or disabled
            // by itself
            tick_next.set(if direction.is_some() {
                tokio::time::sleep(Self::VALUE_TIMEOUT).fuse()
            } else {
                Fuse::terminated()
            });
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/web/input/building/button_blinds_up_down_a")
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
    Down,
    Up,
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
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::Down => &self.signal_down as &dyn signal::Base,
            SignalIdentifier::Up => &self.signal_up as &dyn signal::Base,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct GuiSummary {
    value: Option<bool>,
}
impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let value = *self.value_beat_receiver.borrow();
        GuiSummary { value }
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
                    let value = match request.body_parse_json::<Option<bool>>() {
                        Ok(value) => value,
                        Err(error) => {
                            return async { web::Response::error_400_from_error(error) }.boxed();
                        }
                    };

                    self.value_beat_sender.send(value).unwrap();

                    async { web::Response::ok_empty() }.boxed()
                }
                _ => async { web::Response::error_405() }.boxed(),
            },
            _ => async { web::Response::error_404() }.boxed(),
        }
    }
}
