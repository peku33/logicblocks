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
    pin_mut, select,
    stream::StreamExt,
};
use http::Method;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow, sync::Arc};

pub struct Device<V: DataType + StateValue + Clone + PartialEq> {
    output: state_source::Signal<V>,
    input: state_target::Signal<Option<V>>,
    default: Arc<V>,
}
impl<V: DataType + StateValue + Clone + PartialEq> Device<V> {
    pub fn new(default: V) -> Self {
        let default = Arc::new(default);
        Self {
            output: state_source::Signal::new(default.clone()),
            input: state_target::Signal::new(),
            default,
        }
    }
}
#[async_trait]
impl<V: DataType + StateValue + Clone + PartialEq> DeviceTrait for Device<V> {
    fn get_class(&self) -> Cow<'static, str> {
        format!("soft/state_unwrap<{}>", type_name::<V>()).into()
    }

    fn get_signals(&self) -> Signals {
        hashmap! {
            0 => &self.output as &dyn SignalBase,
            1 => &self.input as &dyn SignalBase,
        }
    }

    async fn run(&self) -> ! {
        let input_runner = self.input.get_stream().for_each(async move |value| {
            let value = match value {
                Some(value) => match &*value {
                    Some(value) => Arc::new(value.clone()),
                    None => self.default.clone(),
                },
                None => self.default.clone(),
            };
            self.output.set(value);
        });
        pin_mut!(input_runner);

        select! {
            _ = input_runner => panic!("input_runner yielded"),
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
