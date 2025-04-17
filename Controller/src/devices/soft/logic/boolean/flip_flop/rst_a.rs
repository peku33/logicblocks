use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
    web::{self, uri_cursor},
};
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    stream::StreamExt,
};
use maplit::hashmap;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Configuration {
    pub initial_value: bool,
}

#[derive(Debug)]
pub struct Device {
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::event_target_last::Signal<bool>,
    signal_r: signal::event_target_last::Signal<()>,
    signal_s: signal::event_target_last::Signal<()>,
    signal_t: signal::event_target_last::Signal<()>,
    signal_output: signal::state_source::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let initial_value = configuration.initial_value;

        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::event_target_last::Signal::<bool>::new(),
            signal_r: signal::event_target_last::Signal::<()>::new(),
            signal_s: signal::event_target_last::Signal::<()>::new(),
            signal_t: signal::event_target_last::Signal::<()>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(Some(initial_value)),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    fn get(&self) -> bool {
        self.signal_output.peek_last().unwrap()
    }
    fn set(
        &self,
        value: bool,
    ) {
        if self.signal_output.set_one(Some(value)) {
            self.signals_sources_changed_waker.wake();
            self.gui_summary_waker.wake();
        }
    }
    fn invert(&self) {
        self.set(!self.get());
    }

    fn signals_targets_changed(&self) {
        let input = self.signal_input.take_pending();
        let r = self.signal_r.take_pending().is_some();
        let s = self.signal_s.take_pending().is_some();
        let t = self.signal_t.take_pending().is_some();

        if let Some(value) = input {
            self.set(value);
        } else {
            match (r, s) {
                (true, true) | (false, false) => {
                    if t {
                        self.invert();
                    }
                }
                (true, false) => {
                    if !t {
                        self.set(false);
                    } else {
                        self.set(true);
                    }
                }
                (false, true) => {
                    if !t {
                        self.set(true);
                    } else {
                        self.set(false);
                    }
                }
            }
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.signals_targets_changed_waker
            .stream()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/boolean/flip_flop/rst_a")
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
    Input,
    R,
    S,
    T,
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
    fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::R => &self.signal_r as &dyn signal::Base,
            SignalIdentifier::S => &self.signal_s as &dyn signal::Base,
            SignalIdentifier::T => &self.signal_t as &dyn signal::Base,
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
        let value = self.signal_output.peek_last().unwrap();

        Self::Value { value }
    }
}

impl uri_cursor::Handler for Device {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor.as_last() {
            Some("r") => match *request.method() {
                http::Method::POST => {
                    self.set(false);
                    async { web::Response::ok_empty() }.boxed()
                }
                _ => async { web::Response::error_405() }.boxed(),
            },
            Some("s") => match *request.method() {
                http::Method::POST => {
                    self.set(true);
                    async { web::Response::ok_empty() }.boxed()
                }
                _ => async { web::Response::error_405() }.boxed(),
            },
            Some("t") => match *request.method() {
                http::Method::POST => {
                    self.invert();
                    async { web::Response::ok_empty() }.boxed()
                }
                _ => async { web::Response::error_405() }.boxed(),
            },
            _ => async { web::Response::error_404() }.boxed(),
        }
    }
}
