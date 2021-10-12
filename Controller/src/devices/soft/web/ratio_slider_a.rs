use crate::{
    datatypes::ratio::Ratio,
    devices, signals,
    signals::signal,
    util::waker_stream,
    web::{self, uri_cursor},
};
use futures::future::{BoxFuture, FutureExt};
use maplit::hashmap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Configuration {
    pub initial: Option<Ratio>,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    gui_summary_waker: waker_stream::mpmc::Sender,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_output: signal::state_source::Signal<Ratio>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let initial = configuration.initial;

        Self {
            configuration,

            gui_summary_waker: waker_stream::mpmc::Sender::new(),

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_output: signal::state_source::Signal::<Ratio>::new(initial),
        }
    }

    fn set(
        &self,
        value: Option<Ratio>,
    ) {
        if self.signal_output.set_one(value) {
            self.signal_sources_changed_waker.wake();
            self.gui_summary_waker.wake();
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/web/ratio_slider_a")
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
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
impl devices::GuiSummaryProvider for Device {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        let gui_summary = self.signal_output.peek_last();
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
                    let value = match request.body_parse_json::<Option<Ratio>>() {
                        Ok(value) => value,
                        Err(error) => {
                            return async move { web::Response::error_400_from_error(error) }
                                .boxed()
                        }
                    };
                    self.set(value);
                    async move { web::Response::ok_empty() }.boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
