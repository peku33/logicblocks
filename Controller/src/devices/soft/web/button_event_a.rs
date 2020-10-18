use crate::{
    devices, signals,
    signals::{
        signal::{self, event_source},
        Signals,
    },
    util::waker_stream,
    web::{self, uri_cursor},
};
use futures::future::{BoxFuture, FutureExt};
use maplit::hashmap;
use std::borrow::Cow;

type SignalOutput = event_source::Signal<()>;

#[derive(Debug)]
pub struct Device {
    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_output: SignalOutput,

    gui_summary_provider_waker: waker_stream::mpmc::Sender,
}
impl Device {
    pub fn click(&self) {
        if self.signal_output.push_one(()) {
            self.signal_sources_changed_waker.wake();
        }
    }
}
impl Device {
    pub fn new() -> Self {
        Self {
            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_output: SignalOutput::new(),

            gui_summary_provider_waker: waker_stream::mpmc::Sender::new(),
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/web/button_event_a")
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
        // Will never be called - no targets
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.signal_output as &dyn signal::Base,
        }
    }
}
impl devices::GuiSummaryProvider for Device {
    fn get_value(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_provider_waker.receiver_factory()
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
                    self.click();
                    async move { web::Response::ok_empty() }.boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
