use crate::{
    datatypes::DataType,
    logic::{
        device::{Device as DeviceTrait, Signals},
        signal::{state_source, state_target, SignalBase, StateValue},
    },
    web::{
        uri_cursor::{Handler, UriCursor},
        Request, Response,
    },
};
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    select,
    stream::StreamExt,
};
use http::Method;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

pub struct Device<V: DataType + StateValue + Clone + PartialEq> {
    output: state_source::Signal<V>,
    input: state_target::Signal<Option<V>>,
    default: V,
}
impl<V: DataType + StateValue + Clone + PartialEq> Device<V> {
    pub fn new(default: V) -> Self {
        Self {
            output: state_source::Signal::new(default.clone()),
            input: state_target::Signal::new(),
            default,
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
            0 => &self.output as &dyn SignalBase,
            1 => &self.input as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let mut input_runner = self
            .input
            .stream()
            .map(|value| {
                value
                    .unwrap_or_else(|| Some(self.default.clone()))
                    .unwrap_or_else(|| self.default.clone())
            })
            .map(Ok)
            .forward(self.output.sink())
            .map(|result| result.unwrap());

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
