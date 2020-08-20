use super::signal::SignalBase;
use crate::web::{
    sse_aggregated::{Node, NodeProvider, PathItem},
    uri_cursor::{Handler, UriCursor},
    Request, Response,
};
use async_trait::async_trait;
use futures::future::{BoxFuture, FutureExt};
use http::Method;
use maplit::hashmap;
use serde_json::json;
use std::{borrow::Cow, collections::HashMap};

pub type SignalId = u16;
pub type Signals<'a> = HashMap<SignalId, &'a dyn SignalBase>;

#[async_trait]
pub trait Device: Sync + Send + Handler + NodeProvider {
    fn class(&self) -> Cow<'static, str>;

    fn signals(&self) -> Signals;

    async fn run(&self) -> !;
    async fn finalize(self: Box<Self>);
}

pub struct DeviceContext<'d> {
    name: String,
    device: Box<dyn Device + 'd>,
}
impl<'d> DeviceContext<'d> {
    pub fn new(
        name: String,
        device: Box<dyn Device + 'd>,
    ) -> Self {
        log::trace!("new called");
        Self { name, device }
    }

    pub fn name(&self) -> &str {
        &self.name
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
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            UriCursor::Terminal => match *request.method() {
                Method::GET => {
                    let response = json!({
                        "name": self.name(),
                        "class": self.device.class(),
                    });
                    async move { Response::ok_json(response) }.boxed()
                }
                _ => async move { Response::error_405() }.boxed(),
            },
            UriCursor::Next("device", uri_cursor) => self.device.handle(request, uri_cursor),
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
impl<'d> NodeProvider for DeviceContext<'d> {
    fn node(&self) -> Node {
        Node::Children(hashmap! {
            PathItem::String("device".to_owned()) => self.device.node()
        })
    }
}
