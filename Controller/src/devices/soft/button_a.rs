use crate::{
    datatypes::void::Void,
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{event_source, SignalBase},
    },
    web::{
        uri_cursor::{Handler, UriCursor},
        Request, Response,
    },
};
use async_trait::async_trait;
use futures::future::{pending as future_pending, BoxFuture, FutureExt};
use http::Method;
use maplit::hashmap;
use std::borrow::Cow;

pub struct Device {
    signal: event_source::Signal<Void>,
}
#[async_trait]
impl DeviceTrait for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/button_a")
    }

    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.signal as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        future_pending().await
    }
    async fn finalize(self: Box<Self>) {}
}
impl Handler for Device {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match (request.method(), uri_cursor.next_item()) {
            (&Method::GET, ("", None)) => async move { Response::ok_empty() }.boxed(),
            (&Method::POST, ("", None)) => {
                self.signal.push(Void::default());
                async move { Response::ok_empty() }.boxed()
            }
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
