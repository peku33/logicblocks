use crate::{
    datatypes::DataType,
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{state_source, state_target, SignalBase, StateValue},
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
use std::{any::type_name, borrow::Cow};

pub struct Device<V: DataType + StateValue + Clone + PartialEq> {
    input: state_target::Signal<Option<V>>,
    output: state_source::Signal<V>,
    default: V,

    sse_sender: waker_stream::Sender,
}
impl<V: DataType + StateValue + Clone + PartialEq> Device<V> {
    pub fn new(default: V) -> Self {
        Self {
            input: state_target::Signal::new(),
            output: state_source::Signal::new(default.clone()),
            default,

            sse_sender: waker_stream::Sender::new(),
        }
    }
}
#[async_trait]
impl<V: DataType + StateValue + Clone + PartialEq> DeviceTrait for Device<V> {
    fn class(&self) -> Cow<'static, str> {
        format!("soft/state_unwrap<{}>", type_name::<V>()).into()
    }

    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.input as &dyn SignalBase,
            1 => &self.output as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let input_runner = self
            .input
            .stream()
            .map(|value| {
                value
                    .unwrap_or_else(|| Some(self.default.clone()))
                    .unwrap_or_else(|| self.default.clone())
            })
            .for_each(async move |value| self.output.set(value));
        pin_mut!(input_runner);

        select! {
            () = input_runner => panic!("input_runner yielded"),
        }
    }
    async fn finalize(self: Box<Self>) {}
}
impl<V: DataType + StateValue + Clone + PartialEq> Handler for Device<V> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            UriCursor::Terminal => match *request.method() {
                Method::GET => {
                    // TODO: Return the actual value
                    async move { Response::ok_empty() }.boxed()
                }
                _ => async move { Response::error_405() }.boxed(),
            },
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
impl<V: DataType + StateValue + Clone + PartialEq> NodeProvider for Device<V> {
    fn node(&self) -> Node {
        Node::Terminal(self.sse_sender.receiver_factory())
    }
}
