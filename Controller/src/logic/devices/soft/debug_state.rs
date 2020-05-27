use super::super::super::{
    device::{Device as DeviceTrait, Signals},
    signal::{state_target::Signal as StateTarget, SignalBase, StateValue},
};
use crate::web::{
    uri_cursor::{Handler, UriCursor},
    Request, Response,
};
use futures::{
    future::{ready, BoxFuture, FutureExt},
    select,
    stream::{pending as stream_pending, BoxStream, StreamExt},
};
use http::Method;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

pub struct Device<V: StateValue + PartialEq + Eq> {
    name: String,
    input: StateTarget<V>,
}
impl<V: StateValue + PartialEq + Eq> Device<V> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            input: StateTarget::new(),
        }
    }
    async fn run(&self) -> ! {
        let mut input_stream = self.input.get_stream();

        loop {
            select! {
                value = input_stream.select_next_some() => {
                    log::debug!("{} -> {:?}", self.name, value);
                }
            }
        }
    }
}
impl<V: StateValue + PartialEq + Eq> DeviceTrait for Device<V> {
    fn get_class(&self) -> Cow<'static, str> {
        format!("soft/DebugState<{}>", type_name::<V>()).into()
    }

    fn get_signals_change_stream(&self) -> BoxStream<()> {
        stream_pending().boxed()
    }
    fn get_signals(&self) -> Signals {
        hashmap! {
            0 => &self.input as &dyn SignalBase,
        }
    }

    fn run(&self) -> BoxFuture<!> {
        self.run().boxed()
    }
    fn finalize(self: Box<Self>) -> BoxFuture<'static, ()> {
        ready(()).boxed()
    }
}
impl<V: StateValue + PartialEq + Eq> Handler for Device<V> {
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
