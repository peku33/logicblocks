use crate::{
    datatypes::{boolean::Boolean, void::Void},
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{event_target, state_source, SignalBase},
    },
    util::waker_stream,
    web::{
        sse_aggregated::{Node, NodeProvider},
        uri_cursor::{Handler, UriCursor},
        Request, Response,
    },
};
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use http::Method;
use maplit::hashmap;
use serde_json::json;
use std::borrow::Cow;

pub struct Device {
    output: state_source::Signal<Boolean>,
    r: event_target::Signal<Void>,
    s: event_target::Signal<Void>,
    t: event_target::Signal<Void>,

    sse_sender: waker_stream::Sender,
}
impl Device {
    pub fn new(initial: Boolean) -> Self {
        Self {
            output: state_source::Signal::new(initial),
            r: event_target::Signal::new(),
            s: event_target::Signal::new(),
            t: event_target::Signal::new(),

            sse_sender: waker_stream::Sender::new(),
        }
    }

    fn r(&self) {
        self.output.set(Boolean::from(false));

        self.sse_sender.wake();
    }
    fn s(&self) {
        self.output.set(Boolean::from(true));

        self.sse_sender.wake();
    }
    fn t(&self) -> bool {
        let value: bool = self.output.current().into();
        let value = !value;
        self.output.set(Boolean::from(value));

        self.sse_sender.wake();

        value
    }
}
#[async_trait]
impl DeviceTrait for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/rst_a")
    }

    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.output as &dyn SignalBase,
            1 => &self.r as &dyn SignalBase,
            2 => &self.s as &dyn SignalBase,
            3 => &self.t as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let r_runner = self.r.stream().for_each(async move |_| {
            self.r();
        });
        pin_mut!(r_runner);

        let s_runner = self.s.stream().for_each(async move |_| {
            self.s();
        });
        pin_mut!(s_runner);

        let t_runner = self.t.stream().for_each(async move |_| {
            self.t();
        });
        pin_mut!(t_runner);

        select! {
            _ = r_runner => panic!("r_runner yielded"),
            _ = s_runner => panic!("s_runner yielded"),
            _ = t_runner => panic!("t_runner yielded"),
        }
    }
    async fn finalize(self: Box<Self>) {}
}
impl Handler for Device {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            UriCursor::Terminal => match *request.method() {
                Method::GET => {
                    let value = self.output.current();
                    async move { Response::ok_json(json!({ "value": value })) }.boxed()
                }
                _ => async move { Response::error_405() }.boxed(),
            },
            UriCursor::Next("r", uri_cursor) => match **uri_cursor {
                UriCursor::Terminal => match *request.method() {
                    Method::POST => {
                        self.r();
                        async move { Response::ok_empty() }.boxed()
                    }
                    _ => async move { Response::error_405() }.boxed(),
                },
                _ => async move { Response::error_404() }.boxed(),
            },
            UriCursor::Next("s", uri_cursor) => match **uri_cursor {
                UriCursor::Terminal => match *request.method() {
                    Method::POST => {
                        self.s();
                        async move { Response::ok_empty() }.boxed()
                    }
                    _ => async move { Response::error_405() }.boxed(),
                },
                _ => async move { Response::error_404() }.boxed(),
            },
            UriCursor::Next("t", uri_cursor) => match **uri_cursor {
                UriCursor::Terminal => match *request.method() {
                    Method::POST => {
                        self.t();
                        async move { Response::ok_empty() }.boxed()
                    }
                    _ => async move { Response::error_405() }.boxed(),
                },
                _ => async move { Response::error_404() }.boxed(),
            },
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
impl NodeProvider for Device {
    fn node(&self) -> Node {
        Node::Terminal(self.sse_sender.receiver_factory())
    }
}
