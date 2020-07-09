use crate::{
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{state_target, SignalBase, StateValue},
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
use serde::Serialize;
use std::{any::type_name, borrow::Cow};

pub struct Device<V: StateValue + Serialize + Clone + PartialEq> {
    name: String,
    input: state_target::Signal<V>,
    sse_sender: waker_stream::Sender,
}
impl<V: StateValue + Serialize + Clone + PartialEq> Device<V> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            input: state_target::Signal::new(),
            sse_sender: waker_stream::Sender::new(),
        }
    }
}
#[async_trait]
impl<V: StateValue + Serialize + Clone + PartialEq> DeviceTrait for Device<V> {
    fn class(&self) -> Cow<'static, str> {
        format!("soft/debug_state<{}>", type_name::<V>()).into()
    }

    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.input as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let input_runner = self
            .input
            .stream()
            .for_each(async move |value| match value {
                Some(value) => {
                    log::debug!("{} -> {:?}", self.name, value);
                }
                None => {
                    log::debug!("{} -> disconnected", self.name);
                }
            });
        pin_mut!(input_runner);

        select! {
            _ = input_runner => panic!("input_runner yielded"),
        }
    }
    async fn finalize(self: Box<Self>) {}
}
impl<V: StateValue + Serialize + Clone + PartialEq> Handler for Device<V> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            UriCursor::Terminal => match *request.method() {
                Method::GET => {
                    let value = self.input.current();
                    async move { Response::ok_json(value) }.boxed()
                }
                _ => async move { Response::error_405() }.boxed(),
            },
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
impl<V: StateValue + Serialize + Clone + PartialEq> NodeProvider for Device<V> {
    fn node(&self) -> Node {
        Node::Terminal(self.sse_sender.receiver_factory())
    }
}
