use crate::{
    datatypes::boolean::Boolean,
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{state_source, state_target, SignalBase},
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
use std::borrow::Cow;

pub struct Device {
    input: state_target::Signal<Boolean>,
    output: state_source::Signal<Option<Boolean>>,

    sse_sender: waker_stream::Sender,
}
impl Device {
    pub fn new() -> Self {
        Self {
            input: state_target::Signal::new(),
            output: state_source::Signal::new(None),

            sse_sender: waker_stream::Sender::new(),
        }
    }
}
#[async_trait]
impl DeviceTrait for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/boolean_invert")
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
            .map(|value| match value {
                None => None,
                Some(value) => {
                    let value: bool = value.into();
                    let value = !value;
                    Some(value.into())
                }
            })
            .for_each(async move |value| {
                self.output.set(value);
            });
        pin_mut!(input_runner);

        select! {
            _ = input_runner => panic!("input_runner yielded"),
        }
    }
    async fn finalize(self: Box<Self>) {}
}
impl Handler for Device {
    fn handle(
        &self,
        _request: Request,
        _uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        async move { Response::error_404() }.boxed()
    }
}
impl NodeProvider for Device {
    fn node(&self) -> Node {
        Node::Terminal(self.sse_sender.receiver_factory())
    }
}
