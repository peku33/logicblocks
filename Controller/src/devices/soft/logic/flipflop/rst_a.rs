use crate::{
    devices,
    signals::{
        self,
        signal::{self, event_target_last, state_source},
        Signals,
    },
    util::waker_stream,
    web::{self, uri_cursor},
};
use async_trait::async_trait;
use futures::{future::BoxFuture, FutureExt};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub initial_value: bool,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    value: bool,
}

#[derive(Debug)]
pub struct Device {
    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_output: state_source::Signal<bool>,
    signal_r: event_target_last::Signal<()>,
    signal_s: event_target_last::Signal<()>,
    signal_t: event_target_last::Signal<()>,

    gui_summary_waker: waker_stream::mpmc::Sender,
}
impl Device {
    pub fn new(
        configuration: Configuration,
        state: Option<State>,
    ) -> Self {
        let state = state.unwrap_or_else(|| State {
            value: configuration.initial_value,
        });

        Self {
            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),

            signal_output: state_source::Signal::new(state.value),
            signal_r: event_target_last::Signal::new(),
            signal_s: event_target_last::Signal::new(),
            signal_t: event_target_last::Signal::new(),

            gui_summary_waker: waker_stream::mpmc::Sender::new(),
        }
    }

    fn handle_inputs(
        &self,
        r: bool,
        s: bool,
        t: bool,
    ) {
        match (r, s) {
            (true, true) | (false, false) => {
                if t {
                    self.t();
                }
            }
            (true, false) => {
                if !t {
                    self.r();
                } else {
                    self.s();
                }
            }
            (false, true) => {
                if !t {
                    self.s();
                } else {
                    self.r();
                }
            }
        }
    }

    pub fn r(&self) {
        if self.signal_output.set_one(false) {
            self.signal_sources_changed_waker.wake();
            self.gui_summary_waker.wake();
        }
    }
    pub fn s(&self) {
        if self.signal_output.set_one(true) {
            self.signal_sources_changed_waker.wake();
            self.gui_summary_waker.wake();
        }
    }
    pub fn t(&self) {
        if self.signal_output.set_one(!self.signal_output.get_last()) {
            self.signal_sources_changed_waker.wake();
            self.gui_summary_waker.wake();
        }
    }
}
#[async_trait]
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/flipflop/rst_a")
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_gui_summary_provider(&self) -> &dyn devices::GuiSummaryProvider {
        self
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        Some(self)
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        self.handle_inputs(
            self.signal_r.take_pending().is_some(),
            self.signal_s.take_pending().is_some(),
            self.signal_t.take_pending().is_some(),
        );
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.signal_output as &dyn signal::Base,
            1 => &self.signal_r as &dyn signal::Base,
            2 => &self.signal_s as &dyn signal::Base,
            3 => &self.signal_t as &dyn signal::Base,
        }
    }
}
#[derive(Serialize)]
struct GuiSummary {
    value: bool,
}
impl devices::GuiSummaryProvider for Device {
    fn get_value(&self) -> Box<dyn devices::GuiSummary> {
        Box::new(GuiSummary {
            value: self.signal_output.get_last(),
        })
    }

    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory {
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
            uri_cursor::UriCursor::Next("r", uri_cursor) => match **uri_cursor {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::POST => {
                        self.r();
                        async move { web::Response::ok_empty() }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next("s", uri_cursor) => match **uri_cursor {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::POST => {
                        self.s();
                        async move { web::Response::ok_empty() }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next("t", uri_cursor) => match **uri_cursor {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::POST => {
                        self.t();
                        async move { web::Response::ok_empty() }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
