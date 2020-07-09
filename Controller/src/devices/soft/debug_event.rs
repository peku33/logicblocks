use crate::{
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{event_target, EventValue, SignalBase},
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
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

pub struct Device<V: EventValue + Clone + PartialEq> {
    name: String,
    input: event_target::Signal<V>,
    sse_sender: waker_stream::Sender,
}
impl<V: EventValue + Clone + PartialEq> Device<V> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            input: event_target::Signal::new(),
            sse_sender: waker_stream::Sender::new(),
        }
    }
}
#[async_trait]
impl<V: EventValue + Clone + PartialEq> DeviceTrait for Device<V> {
    fn class(&self) -> Cow<'static, str> {
        format!("soft/debug_event<{}>", type_name::<V>()).into()
    }

    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.input as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let input_runner = self.input.stream().for_each(async move |value| {
            log::debug!("{} -> {:?}", self.name, value);
        });
        pin_mut!(input_runner);

        select! {
            _ = input_runner => panic!("input_runner yielded"),
        }
    }
    async fn finalize(self: Box<Self>) {}
}
impl<V: EventValue + Clone + PartialEq> Handler for Device<V> {
    fn handle(
        &self,
        _request: Request,
        _uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        async move { Response::error_404() }.boxed()
    }
}
impl<V: EventValue + Clone + PartialEq> NodeProvider for Device<V> {
    fn node(&self) -> Node {
        Node::Terminal(self.sse_sender.receiver_factory())
    }
}
