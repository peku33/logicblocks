use crate::{
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{state_target, SignalBase, StateValue},
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
use std::{any::type_name, borrow::Cow};

pub struct Device<V: StateValue + PartialEq> {
    name: String,
    input: state_target::Signal<V>,
}
impl<V: StateValue + PartialEq> Device<V> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            input: state_target::Signal::new(),
        }
    }
}
#[async_trait]
impl<V: StateValue + PartialEq> DeviceTrait for Device<V> {
    fn get_class(&self) -> Cow<'static, str> {
        format!("soft/debug_state<{}>", type_name::<V>()).into()
    }

    fn get_signals(&self) -> Signals {
        hashmap! {
            0 => &self.input as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let input_runner = self
            .input
            .get_stream()
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
impl<V: StateValue + PartialEq> Handler for Device<V> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match (request.method(), uri_cursor.next_item()) {
            (&Method::GET, ("", None)) => async move {
                // TODO: Return the actual value
                Response::ok_empty()
            }
            .boxed(),
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
