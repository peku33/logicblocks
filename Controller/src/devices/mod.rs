pub mod houseblocks;
pub mod runner;
pub mod soft;

use crate::{
    signals,
    util::waker_stream,
    web::{self, sse_aggregated, uri_cursor},
};
use async_trait::async_trait;
use futures::future::{pending, BoxFuture, FutureExt};
use maplit::hashmap;
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

pub struct DeviceHandler<'d> {
    name: String,
    device: Box<dyn Device + 'd>,
}
impl<'d> DeviceHandler<'d> {
    pub fn new(
        name: String,
        device: Box<dyn Device + 'd>,
    ) -> Self {
        Self { name, device }
    }

    pub fn name(&self) -> &String {
        &self.name
    }
    fn device(&self) -> &(dyn Device + 'd) {
        &*self.device
    }

    pub fn gui_summary_waker(&self) -> sse_aggregated::Node {
        sse_aggregated::Node {
            terminal: Some(self.device().as_gui_summary_provider().get_waker()),
            children: hashmap! {},
        }
    }

    pub fn close(self) -> Box<dyn Device + 'd> {
        self.device
    }
}
impl<'d> uri_cursor::Handler for DeviceHandler<'d> {
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

pub trait GuiSummary = erased_serde::Serialize + Send + Sync;
pub trait GuiSummaryProvider {
    fn get_value(&self) -> Box<dyn GuiSummary>;
    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory;
}
