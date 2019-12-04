use super::device::{DeviceTrait, RunObjectTrait};
use super::device_event_stream;
use crate::util::bus2;
use crate::util::ref_mut_async::FutureWrapper;
use crate::web::router::uri_cursor::{Handler, UriCursor};
use crate::web::sse;
use crate::web::{Request, Response};
use failure::{err_msg, format_err, Error};
use futures::future::{pending, ready, BoxFuture, FutureExt};
use futures::select;
use futures::stream::{Stream, StreamExt};
use owning_ref::OwningHandle;
use serde_json::json;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

pub type DeviceId = u64;
#[derive(Clone, Debug)]
pub struct EventStreamItem {
    device_id: DeviceId,
    device_event: device_event_stream::Item,
}

pub struct Pool {
    device_id: DeviceId,
    devices:
        HashMap<DeviceId, OwningHandle<Box<dyn DeviceTrait>, Box<dyn RunObjectTrait<'static>>>>,

    event_stream_sender: RefCell<bus2::Sender<EventStreamItem>>,
    event_stream_receiver_factory: bus2::ReceiverFactory<EventStreamItem>,
}
impl Pool {
    pub fn new() -> Self {
        let (event_stream_sender, event_stream_receiver_factory) = bus2::channel();
        let event_stream_sender = RefCell::new(event_stream_sender);

        return Self {
            device_id: 0,
            devices: HashMap::new(),

            event_stream_sender,
            event_stream_receiver_factory,
        };
    }
    pub fn add(
        &mut self,
        device: Box<dyn DeviceTrait>,
    ) -> DeviceId {
        let device_owning_handle =
            OwningHandle::new_with_fn(device, unsafe { |device_ptr| (*device_ptr).device_run() });
        self.device_id += 1;
        let devices_insert_result = self
            .devices
            .insert(self.device_id, device_owning_handle)
            .is_none();
        if !devices_insert_result {
            panic!("Duplicated device");
        }
        return self.device_id;
    }
    pub async fn run(&self) -> Error {
        let (device_id, error) = self
            .devices
            .iter()
            .map(|(device_id, device_owning_handle)| {
                return async move {
                    let run_future =
                        FutureWrapper::new(device_owning_handle.get_run_future().borrow_mut());
                    let event_stream_forward_future =
                        match device_owning_handle.event_stream_subscribe() {
                            Some(event_stream_future) => event_stream_future
                                .for_each(|device_event| {
                                    let event_stream_item = EventStreamItem {
                                        device_id: *device_id,
                                        device_event,
                                    };
                                    self.event_stream_sender.borrow_mut().send(event_stream_item);
                                    return ready(());
                                })
                                .boxed_local(),
                            None => pending().boxed_local(),
                        };

                    let error = select!(
                        run_future_error = run_future.fuse() => err_msg("run_future"),
                        event_stream_forward_future_error = event_stream_forward_future.fuse() => err_msg("event_stream_forward_future"),
                    );
                    return (device_id, error);
                };
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .next()
            .await
            .unwrap();

        return format_err!(
            "device_id = {:?} exited with error = {:?}",
            device_id,
            error
        );
    }
    pub fn get_event_stream_receiver(&self) -> impl Stream<Item = EventStreamItem> {
        return self.event_stream_receiver_factory.receiver();
    }
    fn get_sse_response_stream(&self) -> impl Stream<Item = sse::Event> {
        return self.get_event_stream_receiver().map(|event_stream_item| {
            return sse::Event {
                id: Some(Cow::from(event_stream_item.device_id.to_string())),
                data: event_stream_item.device_event,
                ..sse::Event::default()
            };
        });
    }
}
impl Handler for Pool {
    fn handle(
        &self,
        request: &Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor.next_item() {
            ("", None) => {
                let devices = self
                    .devices
                    .iter()
                    .map(|(device_id, device_owning_handle)| {
                        return (
                            *device_id,
                            device_owning_handle.as_owner().device_class_get(),
                        );
                    })
                    .collect::<Vec<_>>();

                return async move {
                    return Response::from_json(
                        devices
                            .iter()
                            .map(|(device_id, device_class)| {
                                json!({
                                    "deviceId": device_id,
                                    "deviceClass": device_class,
                                })
                            })
                            .collect(),
                    );
                }
                .boxed();
            }
            ("event_stream", None) => {
                return ready(Response::from_sse_stream(self.get_sse_response_stream())).boxed();
            }
            (device_id, uri_cursor) => {
                let uri_cursor = match uri_cursor {
                    Some(uri_cursor) => uri_cursor,
                    None => return ready(Response::error_404()).boxed(),
                };
                let device_id: DeviceId = match device_id.parse() {
                    Ok(device_id) => device_id,
                    Err(_) => {
                        return ready(Response::error_404()).boxed();
                    }
                };
                let device = match self.devices.get(&device_id) {
                    Some(device) => device.as_owner(),
                    None => {
                        return ready(Response::error_404()).boxed();
                    }
                };
                let device_routed_handler = match device.device_as_routed_handler() {
                    Some(device_routed_handler) => device_routed_handler,
                    None => {
                        return ready(Response::error_404()).boxed();
                    }
                };
                return device_routed_handler.handle(request, uri_cursor);
            }
        }
    }
}