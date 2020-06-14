use super::api::{Client as ApiClient, Stream};
use super::events::{EventSource, EventStreamBuilder, EventsTracker};
use crate::devices::device::{DeviceTrait, RunObjectTrait};
use crate::devices::device_event_stream;
use crate::devices::soft::ipc::snapshot::Driver as SnapshotDriver;
use crate::web::uri_cursor::{Handler, UriCursor};
use crate::web::{Request, Response};
use failure::{err_msg, Error};
use futures::future::{ready, BoxFuture, FutureExt, LocalBoxFuture};
use futures::stream::StreamExt;
use futures::{pin_mut, select};
use owning_ref::OwningHandle;
use serde::Serialize;
use serde_json::json;
use std::cell::RefCell;
use std::time::Duration;

#[derive(Serialize, Clone, Copy, Debug)]
pub enum State {
    Initializing,
    Configuring,
    Running,
    Error,
}

struct ApiClientDependencies {
    snapshot_driver: SnapshotDriver<'static>,
}
pub struct Device {
    device_name: String,
    shared_user_password: String,

    api_client_and_dependencies: OwningHandle<Box<ApiClient>, Box<ApiClientDependencies>>,
    event_stream_builder: EventStreamBuilder,

    state: RefCell<State>,
    events_tracker: RefCell<EventsTracker>,
}
impl Device {
    const DEVICE_CLASS: &'static str = "dahua/ipc";

    pub fn new(
        host: http::uri::Authority,
        admin_password: String,
        device_name: String,
        shared_user_password: String,
    ) -> Self {
        let api_client = ApiClient::new(host.clone(), admin_password.clone());
        let api_client_and_dependencies =
            OwningHandle::new_with_fn(Box::new(api_client), |api_client_ptr| {
                Box::new(ApiClientDependencies {
                    snapshot_driver: SnapshotDriver::new(
                        unsafe { Box::new(move || (*api_client_ptr).snapshot().boxed_local()) },
                        Duration::from_secs(60),
                    ),
                })
            });

        let event_stream_builder =
            EventStreamBuilder::new(host, "admin".to_owned(), admin_password);

        let state = RefCell::new(State::Initializing);

        let events_tracker = RefCell::new(EventsTracker::new());

        Self {
            shared_user_password,
            device_name,

            api_client_and_dependencies,
            event_stream_builder,

            state,
            events_tracker,
        }
    }
    async fn run_once(
        &self,
        device_event_stream_sender: &device_event_stream::Sender,
    ) -> Error {
        self.state.replace(State::Initializing);
        self.api_client_and_dependencies.snapshot_driver.reset();
        self.events_tracker.borrow_mut().clear();
        device_event_stream_sender.send_empty();

        // API
        self.state.replace(State::Configuring);
        device_event_stream_sender.send_empty();
        // let sane_defaults_config = SaneDefaultsConfig {
        //     device_name: self.device_name.clone(),
        //     shared_user_password: self.shared_user_password.clone(),
        //     video_overlay: Some(self.device_name.clone()),
        // };
        // if let Err(error) = self
        //     .api_client_and_dependencies
        //     .as_owner()
        //     .sane_defaults(&sane_defaults_config)
        //     .await
        // {
        //     return error;
        // }

        // TODO: Recorder

        // Snapshot driver
        let snapshot_driver_future = self
            .api_client_and_dependencies
            .snapshot_driver
            .run()
            .for_each(|()| {
                device_event_stream_sender.send_str("snapshot");
                ready(())
            });
        pin_mut!(snapshot_driver_future);

        // Events Tracker
        let device_event_stream = match self.event_stream_builder.get_event_stream().await {
            Ok(device_event_stream) => device_event_stream,
            Err(error) => return error,
        }
        .for_each(|event_transition| {
            self.events_tracker
                .borrow_mut()
                .consume_event_transition(event_transition);
            device_event_stream_sender.send_empty();
            ready(())
        });
        pin_mut!(device_event_stream);

        // Running
        self.state.replace(State::Running);
        device_event_stream_sender.send_empty();

        // Error handling
        return select! {
            snapshot_driver_future_error = snapshot_driver_future.fuse() => err_msg("snapshot_driver_future exited"),
            device_event_stream_error = device_event_stream.fuse() => err_msg("device_event_stream exited"),
        };
    }
    async fn run_loop(
        &self,
        device_event_stream_sender: &device_event_stream::Sender,
    ) {
        loop {
            let error: Error = self.run_once(device_event_stream_sender).await;
            log::error!("run_once error: {}", error);

            self.state.replace(State::Error);
            self.api_client_and_dependencies.snapshot_driver.reset();
            self.events_tracker.borrow_mut().clear();
            device_event_stream_sender.send_empty();

            tokio::time::delay_for(Duration::from_secs(60)).await;
        }
    }
}
impl DeviceTrait for Device {
    fn device_class_get(&self) -> &'static str {
        Device::DEVICE_CLASS
    }
    fn device_run<'s>(&'s self) -> Box<dyn RunObjectTrait<'s> + 's> {
        let (device_event_stream_sender, device_event_stream_receiver_factory) =
            device_event_stream::channel();
        Box::new(RunObject {
            run_future: RefCell::new(
                async move {
                    return self.run_loop(&device_event_stream_sender).await;
                }
                .boxed_local(),
            ),
            device_event_stream_receiver_factory,
        })
    }
    fn device_as_routed_handler(&self) -> Option<&dyn Handler> {
        Some(self)
    }
}
impl Handler for Device {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        match (request.method(), uri_cursor.next_item()) {
            (&http::Method::GET, ("", None)) => {
                let device_name = self.device_name.clone();
                let state = *self.state.borrow();
                let snapshot_available =
                    self.api_client_and_dependencies.snapshot_driver.has_image();
                let events: Vec<EventSource> =
                    self.events_tracker.borrow().iter().cloned().collect();

                let rtsp_stream_main = self
                    .api_client_and_dependencies
                    .as_owner()
                    .get_stream_rtsp_uri(Stream::Main, &self.shared_user_password)
                    .into_string();

                let rtsp_stream_sub1 = self
                    .api_client_and_dependencies
                    .as_owner()
                    .get_stream_rtsp_uri(Stream::Sub1, &self.shared_user_password)
                    .into_string();

                let rtsp_stream_sub2 = self
                    .api_client_and_dependencies
                    .as_owner()
                    .get_stream_rtsp_uri(Stream::Sub2, &self.shared_user_password)
                    .into_string();

                async move {
                    return Response::ok_json(json!({
                        "device_name": device_name,
                        "state": state,
                        "snapshot_available": snapshot_available,
                        "events": events,
                        "rtsp_streams": {
                            "main": rtsp_stream_main,
                            "sub1": rtsp_stream_sub1,
                            "sub2": rtsp_stream_sub2,
                        },
                    }));
                }
                .boxed()
            }
            (_, ("snapshot", Some(uri_cursor))) => self
                .api_client_and_dependencies
                .snapshot_driver
                .handle(request, uri_cursor),
            _ => ready(Response::error_404()).boxed(),
        }
    }
}

pub struct RunObject<'d> {
    run_future: RefCell<LocalBoxFuture<'d, ()>>,
    device_event_stream_receiver_factory: device_event_stream::ReceiverFactory,
}
impl<'d> RunObjectTrait<'d> for RunObject<'d> {
    fn get_run_future(&self) -> &RefCell<LocalBoxFuture<'d, ()>> {
        &self.run_future
    }
    fn event_stream_subscribe(&self) -> Option<device_event_stream::Receiver> {
        Some(self.device_event_stream_receiver_factory.receiver())
    }
}
