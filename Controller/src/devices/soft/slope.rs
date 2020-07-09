use crate::{
    datatypes::{boolean::Boolean, void::Void},
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{event_source, state_target, SignalBase},
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
    output_false: event_source::Signal<Void>,
    output_true: event_source::Signal<Void>,

    sse_sender: waker_stream::Sender,
}
impl Device {
    pub fn new() -> Self {
        Self {
            input: state_target::Signal::new(),
            output_false: event_source::Signal::new(),
            output_true: event_source::Signal::new(),

            sse_sender: waker_stream::Sender::new(),
        }
    }
}
#[async_trait]
impl DeviceTrait for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/slope")
    }

    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.input as &dyn SignalBase,
            1 => &self.output_false as &dyn SignalBase,
            2 => &self.output_true as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let input_runner = self.input.stream().for_each(async move |value| {
            if let Some(value) = value {
                match value.into() {
                    false => {
                        self.output_false.push(Void::default());
                    }
                    true => {
                        self.output_true.push(Void::default());
                    }
                }
            }
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
