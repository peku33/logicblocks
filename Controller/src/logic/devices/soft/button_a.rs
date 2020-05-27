use super::super::super::{
    device::{Device as DeviceTrait, Signals},
    signal::{event_source::Signal as EventSource, SignalBase},
    signal_values::Void,
};
use crate::web::{
    uri_cursor::{Handler, UriCursor},
    Request, Response,
};
use futures::{
    future::{pending as future_pending, ready, BoxFuture, FutureExt},
    stream::{pending as stream_pending, BoxStream, StreamExt},
};
use http::Method;
use maplit::hashmap;
use std::borrow::Cow;

pub struct Device {
    signal: EventSource<Void>,
}
impl DeviceTrait for Device {
    fn get_class(&self) -> Cow<'static, str> {
        Cow::from("soft/Button/A")
    }

    fn get_signals_change_stream(&self) -> BoxStream<()> {
        stream_pending().boxed()
    }
    fn get_signals(&self) -> Signals {
        hashmap! {
            0 => &self.signal as &dyn SignalBase,
        }
    }

    fn run(&self) -> BoxFuture<!> {
        future_pending().boxed()
    }
    fn finalize(self: Box<Self>) -> BoxFuture<'static, ()> {
        ready(()).boxed()
    }
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
                self.signal.push(Void::new().into());
                async move { Response::ok_empty() }.boxed()
            }
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
