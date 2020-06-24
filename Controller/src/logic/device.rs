use super::signal::SignalBase;
use crate::web::{
    uri_cursor::{Handler, UriCursor},
    Request, Response,
};
use async_trait::async_trait;
use futures::future::{BoxFuture, FutureExt};
use http::Method;
use serde_json::json;
use std::{borrow::Cow, collections::HashMap};

pub type SignalId = u16;
pub type Signals<'a> = HashMap<SignalId, &'a dyn SignalBase>;

#[async_trait]
pub trait Device: Sync + Send + Handler {
    fn class(&self) -> Cow<'static, str>;

    fn signals(&self) -> Signals;

    async fn run(&self) -> !;
    async fn finalize(self: Box<Self>);
}

pub struct DeviceContext<'d> {
    device: Box<dyn Device + 'd>,
}
impl<'d> DeviceContext<'d> {
    pub fn new(device: Box<dyn Device + 'd>) -> Self {
        log::trace!("new called");
        Self { device }
    }

    pub fn as_device(&self) -> &dyn Device {
        &*self.device
    }
    pub fn into_device(self) -> Box<dyn Device + 'd> {
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
impl<'d> Handler for DeviceContext<'d> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match (request.method(), uri_cursor.next_item()) {
            // Shared device information
            (&Method::GET, ("", None)) => {
                let response = json!({
                    "class": self.device.class(),
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
