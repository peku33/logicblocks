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
use std::{borrow::Cow, time::Duration};

const SHORT_DURATION: Duration = Duration::from_secs(2);

pub struct Device {
    input: state_target::Signal<Boolean>,
    output_short: event_source::Signal<Void>,
    output_long: event_source::Signal<Void>,

    sse_sender: waker_stream::Sender,
}
impl Device {
    pub fn new() -> Self {
        Self {
            input: state_target::Signal::new(),
            output_short: event_source::Signal::new(),
            output_long: event_source::Signal::new(),

            sse_sender: waker_stream::Sender::new(),
        }
    }
}
#[async_trait]
impl DeviceTrait for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/button_controller_a")
    }

    fn signals(&self) -> Signals {
        hashmap! {
            0 => &self.input as &dyn SignalBase,
            1 => &self.output_short as &dyn SignalBase,
            2 => &self.output_long as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let input_runner = async move {
            let mut input_stream = self.input.stream();
            loop {
                // Wait for key press
                match input_stream.select_next_some().await {
                    None => continue, // Detached input
                    Some(value) => match value.into() {
                        false => continue, // Depressed key, probably from previous iteration
                        true => (),        // Go to next step
                    },
                };

                // Either wait for key depress or timer completion
                // false - user depressed
                // true - timer expired
                let mut timer_runner = tokio::time::delay_for(SHORT_DURATION).map(|()| true);
                let mut button_runner = input_stream.select_next_some().map(|value| match value {
                    None => true, // Button detached
                    Some(value) => match value.into() {
                        false => false, // User depressed
                        true => true,   // This should never happen
                    },
                });

                let expired = select! {
                    expired = timer_runner => expired,
                    expired = button_runner => expired,
                };

                match expired {
                    false => self.output_short.push(Void::default()),
                    true => self.output_long.push(Void::default()),
                };
            }
        };
        pin_mut!(input_runner);
        let mut input_runner = input_runner.fuse();

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
