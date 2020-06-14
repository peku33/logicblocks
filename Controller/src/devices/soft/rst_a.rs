use crate::{
    datatypes::{boolean::Boolean, void::Void},
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{event_target, state_source, SignalBase},
    },
    web::{
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
}
impl Device {
    pub fn new(initial: Boolean) -> Self {
        Self {
            output: state_source::Signal::new(initial.into()),
            r: event_target::Signal::new(),
            s: event_target::Signal::new(),
            t: event_target::Signal::new(),
        }
    }

    fn r(&self) {
        self.output.set(Boolean::from(false).into());
    }
    fn s(&self) {
        self.output.set(Boolean::from(true).into());
    }
    fn t(&self) -> bool {
        let value: bool = (*self.output.get()).into();
        let value = !value;
        self.output.set(Boolean::from(value).into());
        value
    }
}
#[async_trait]
impl DeviceTrait for Device {
    fn get_class(&self) -> Cow<'static, str> {
        Cow::from("soft/rst_a")
    }

    fn get_signals(&self) -> Signals {
        hashmap! {
            0 => &self.output as &dyn SignalBase,
            1 => &self.r as &dyn SignalBase,
            2 => &self.s as &dyn SignalBase,
            3 => &self.t as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let r_runner = self.r.get_stream().for_each(async move |_| {
            self.r();
        });
        pin_mut!(r_runner);

        let s_runner = self.s.get_stream().for_each(async move |_| {
            self.s();
        });
        pin_mut!(s_runner);

        let t_runner = self.t.get_stream().for_each(async move |_| {
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
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match (request.method(), uri_cursor.next_item()) {
            (&Method::GET, ("", None)) => {
                let value: bool = (*self.output.get()).into();
                async move { Response::ok_json(json!({ "value": value })) }.boxed()
            }
            (&Method::POST, ("r", None)) => {
                self.r();
                async move { Response::ok_json(false) }.boxed()
            }
            (&Method::POST, ("s", None)) => {
                self.s();
                async move { Response::ok_json(true) }.boxed()
            }
            (&Method::POST, ("t", None)) => {
                let value = self.t();
                async move { Response::ok_json(value) }.boxed()
            }
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
