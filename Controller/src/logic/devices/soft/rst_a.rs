use super::super::super::{
    device::{Device as DeviceTrait, Signals},
    signal::{
        event_target::Signal as EventTarget, state_source::Signal as StateSource, SignalBase,
    },
    signal_values::{Bool, Void},
};
use crate::web::{
    uri_cursor::{Handler, UriCursor},
    Request, Response,
};
use futures::{
    future::{ready, BoxFuture, FutureExt},
    select,
    stream::{pending as stream_pending, BoxStream, StreamExt},
};
use http::Method;
use maplit::hashmap;
use serde_json::json;
use std::borrow::Cow;

pub struct Device {
    r: EventTarget<Void>,
    s: EventTarget<Void>,
    t: EventTarget<Void>,
    output: StateSource<Bool>,
}
impl Device {
    pub fn new(initial: Bool) -> Self {
        Self {
            r: EventTarget::new(),
            s: EventTarget::new(),
            t: EventTarget::new(),
            output: StateSource::new(initial.into()),
        }
    }
    async fn run(&self) -> ! {
        let mut r_stream = self.r.get_stream();
        let mut s_stream = self.s.get_stream();
        let mut t_stream = self.t.get_stream();

        loop {
            select! {
                Void = r_stream.select_next_some() => {
                    self.r();
                },
                Void = s_stream.select_next_some() => {
                    self.s();
                },
                Void = t_stream.select_next_some() => {
                    self.t();
                },
            }
        }
    }

    fn r(&self) {
        self.output.set(Bool::new(false).into());
    }
    fn s(&self) {
        self.output.set(Bool::new(true).into());
    }
    fn t(&self) {
        self.output.set(Bool::new(self.output.get().value()).into());
    }
}
impl DeviceTrait for Device {
    fn get_class(&self) -> Cow<'static, str> {
        Cow::from("soft/RST/A")
    }

    fn get_signals_change_stream(&self) -> BoxStream<()> {
        stream_pending().boxed()
    }
    fn get_signals(&self) -> Signals {
        hashmap! {
            0 => &self.r as &dyn SignalBase,
            1 => &self.s as &dyn SignalBase,
            2 => &self.t as &dyn SignalBase,
            3 => &self.output as &dyn SignalBase,
        }
    }

    fn run(&self) -> BoxFuture<!> {
        self.run().boxed()
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
            (&Method::GET, ("", None)) => {
                let value = self.output.get().value();
                async move { Response::ok_json(json!({ "value": value })) }.boxed()
            }
            (&Method::POST, ("r", None)) => {
                self.r();
                async move { Response::ok_empty() }.boxed()
            }
            (&Method::POST, ("s", None)) => {
                self.s();
                async move { Response::ok_empty() }.boxed()
            }
            (&Method::POST, ("t", None)) => {
                self.t();
                async move { Response::ok_empty() }.boxed()
            }
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
