pub mod houseblocks;
pub mod runner;
pub mod soft;

use crate::{
    signals,
    util::{atomic_cell::AtomicCell, waker_stream},
    web::{self, sse_aggregated, uri_cursor},
};
use async_trait::async_trait;
use futures::future::{pending, BoxFuture, FutureExt};
use maplit::hashmap;
use owning_ref::OwningHandle;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;
use std::{borrow::Cow, fmt};

pub type Id = u32;

pub trait Configuration = Serialize + DeserializeOwned;
pub trait State = Serialize + DeserializeOwned;

#[async_trait]
pub trait Device: Send + Sync + fmt::Debug {
    fn class(&self) -> Cow<'static, str>;

    fn as_signals_device(&self) -> &dyn signals::Device;
    fn as_gui_summary_provider(&self) -> &dyn GuiSummaryProvider;
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        None
    }

    async fn run(&self) -> ! {
        pending().await
    }
    async fn finalize(&self) {}
}

struct DeviceContextInner<'d> {
    run_future: AtomicCell<BoxFuture<'d, !>>,
}
pub struct DeviceContext<'d> {
    name: String,
    inner: OwningHandle<Box<dyn Device + 'd>, Box<DeviceContextInner<'d>>>,
}

impl<'d> DeviceContext<'d> {
    pub fn new(
        name: String,
        device: Box<dyn Device + 'd>,
    ) -> Self {
        let inner = OwningHandle::new_with_fn(device, |device_ptr| unsafe {
            let run_future = AtomicCell::new((*device_ptr).run());
            Box::new(DeviceContextInner { run_future })
        });
        Self { name, inner }
    }

    pub fn name(&self) -> &String {
        &self.name
    }
    fn device(&self) -> &(dyn Device + 'd) {
        &**self.inner.as_owner()
    }

    pub fn gui_summary_waker(&self) -> sse_aggregated::Node {
        sse_aggregated::Node {
            terminal: Some(self.device().as_gui_summary_provider().get_waker()),
            children: hashmap! {},
        }
    }

    // Could be called many times
    pub async fn run(&self) -> ! {
        let mut run_future = self.inner.run_future.lease();
        (&mut *run_future).await
    }
    pub async fn finalize(&self) {
        self.inner.as_owner().finalize().await;
    }

    pub fn close(self) -> Box<dyn Device + 'd> {
        self.inner.into_owner()
    }
}
impl<'d> uri_cursor::Handler for DeviceContext<'d> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::GET => {
                    let name = self.name().clone();
                    let class = self.device().class();
                    async move {
                        let response = json!({
                            "name": name,
                            "class": class,
                        });
                        web::Response::ok_json(response)
                    }
                    .boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            uri_cursor::UriCursor::Next("gui-summary", uri_cursor) => match **uri_cursor {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::GET => {
                        let value = self.device().as_gui_summary_provider().get_value();
                        async move { web::Response::ok_json(value) }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next("device", uri_cursor) => {
                match self.device().as_web_handler() {
                    Some(handler) => handler.handle(request, uri_cursor),
                    None => async move { web::Response::error_404() }.boxed(),
                }
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}

pub trait GuiSummaryProvider {
    fn get_value(&self) -> serde_json::Value;
    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory;
}
