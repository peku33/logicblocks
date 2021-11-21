use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
    },
    web::{self, uri_cursor},
};
use async_trait::async_trait;
use futures::future::{BoxFuture, FutureExt};
use maplit::hashmap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Device {
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_output: signal::event_source::Signal<bool>,
}
impl Device {
    pub fn new() -> Self {
        Self {
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_output: signal::event_source::Signal::<bool>::new(),
        }
    }

    pub fn push(
        &self,
        value: bool,
    ) {
        if self.signal_output.push_one(value) {
            self.signals_sources_changed_waker.wake();
        }
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/web/button_event_boolean_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
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
        exit_flag.await;
        Exited
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

                    self.push(value);

                    async move { web::Response::ok_empty() }.boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
