use super::signal::SignalBase;
use crate::web::{
    uri_cursor::{Handler, UriCursor},
    Request, Response,
};
use futures::{
    future::{BoxFuture, FutureExt},
    stream::BoxStream,
};
use http::Method;
use serde_json::json;
use std::{borrow::Cow, collections::HashMap};

pub type SignalId = u16;
pub type Signals<'a> = HashMap<SignalId, &'a dyn SignalBase>;

pub trait Device: Sync + Send + Handler {
    fn get_class(&self) -> Cow<'static, str>;

    fn get_signals_change_stream(&self) -> BoxStream<()>;
    fn get_signals(&self) -> Signals;

    fn run(&self) -> BoxFuture<!>;
    fn finalize(self: Box<Self>) -> BoxFuture<'static, ()>;
}

pub struct DeviceContext {
    device: Box<dyn Device>,
}
impl DeviceContext {
    pub fn new(device: Box<dyn Device>) -> Self {
        log::trace!("new called");
        Self { device }
    }

    pub fn get_device(&self) -> &dyn Device {
        &*self.device
    }
    pub fn into_device(self) -> Box<dyn Device> {
        self.device
    }

    pub async fn run(&self) -> ! {
        log::trace!("run called");

        self.device.run().await
    }

    pub async fn finalize(self) {
        log::trace!("finalize begin");

        self.device.finalize().await;

        log::trace!("finalize end");
    }
}
impl Handler for DeviceContext {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match (request.method(), uri_cursor.next_item()) {
            // Shared device information
            (&Method::GET, ("", None)) => {
                let response = json!({
                    "class": self.device.get_class(),
                });

                async move { Response::ok_json(response) }.boxed()
            }
            // Device dependant endpoint
            (_, ("device", Some(uri_cursor_next_item))) => {
                self.device.handle(request, uri_cursor_next_item)
            }
            // Others
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
