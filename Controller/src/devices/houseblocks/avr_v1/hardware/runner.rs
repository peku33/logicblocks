use super::{
    super::super::houseblocks_v1::{
        common::{Address, AddressDeviceType, AddressSerial},
        master::Master,
    },
    driver::{ApplicationDriver, Driver},
    property,
};
use crate::{
    util::{optional_async::StreamOrPending, waker_stream},
    web::{self, sse_aggregated, uri_cursor},
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use futures::{
    future::{pending, BoxFuture, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use maplit::hashmap;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;
use std::{cmp::min, collections::HashMap, fmt, time::Duration};

#[async_trait]
pub trait BusDevice {
    async fn initialize(
        &self,
        driver: &ApplicationDriver<'_>,
    ) -> Result<(), Error>;

    fn poll_delay(&self) -> Option<Duration>;
    async fn poll(
        &self,
        driver: &ApplicationDriver<'_>,
    ) -> Result<(), Error>;

    async fn deinitialize(
        &self,
        driver: &ApplicationDriver<'_>,
    ) -> Result<(), Error>;

    fn failed(&self);
}

pub trait Properties {
    fn by_name(&self) -> HashMap<&'static str, &dyn property::Base>;

    fn in_any_user_pending(&self) -> bool;

    type Remote: Sync + Send;
    fn remote(&self) -> Self::Remote;
}

#[async_trait]
pub trait Device: BusDevice + Sync + Send + Sized + fmt::Debug {
    fn new() -> Self;

    fn device_type_name() -> &'static str;
    fn address_device_type() -> AddressDeviceType;

    type Properties: Properties;
    fn properties(&self) -> &Self::Properties;

    fn poll_waker_receiver(&self) -> Option<waker_stream::mpsc::ReceiverLease> {
        None
    }
    async fn run(&self) -> ! {
        pending().await
    }
    async fn finalize(&self) {}
}

#[derive(Serialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DeviceState {
    ERROR,
    INITIALIZING,
    RUNNING,
}

const POLL_DELAY_MAX: Duration = Duration::from_secs(5);
const ERROR_RESTART_DELAY: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct Runner<'m, D: Device> {
    driver: Driver<'m>,
    device: D,

    device_state: Mutex<DeviceState>,

    properties_remote_in_change_waker: waker_stream::mpsc::SenderReceiver,
    properties_remote_out_change_waker: waker_stream::mpsc::SenderReceiver,

    sse_aggregated_waker_stream: waker_stream::mpmc::Sender,
}
impl<'m, D: Device> Runner<'m, D> {
    pub fn new(
        master: &'m Master,
        address_serial: AddressSerial,
    ) -> Self {
        let driver = Driver::new(
            master,
            Address::new(D::address_device_type(), address_serial),
        );
        let device = D::new();

        let device_state = Mutex::new(DeviceState::INITIALIZING);

        Self {
            driver,
            device,

            device_state,

            properties_remote_in_change_waker: waker_stream::mpsc::SenderReceiver::new(),
            properties_remote_out_change_waker: waker_stream::mpsc::SenderReceiver::new(),

            sse_aggregated_waker_stream: waker_stream::mpmc::Sender::new(),
        }
    }

    pub fn properties_remote(&self) -> <D::Properties as Properties>::Remote {
        self.device.properties().remote()
    }
    pub fn properties_remote_in_change_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.properties_remote_in_change_waker.receiver()
    }
    pub fn properties_remote_out_change_waker_wake(&self) {
        self.properties_remote_out_change_waker.wake();
    }

    async fn driver_run_once(&self) -> Error {
        *self.device_state.lock() = DeviceState::INITIALIZING;
        self.sse_aggregated_waker_stream.wake();

        // Hardware initializing & avr_v1
        if let Err(error) = self.driver.prepare().await.context("initial prepare") {
            return error;
        }

        // Device is prepared in application mode, we can start application driver
        let application_driver = ApplicationDriver::new(&self.driver);

        // User mode initializing
        if let Err(error) = self
            .device
            .initialize(&application_driver)
            .await
            .context("initialize")
        {
            return error;
        }
        if self.device.properties().in_any_user_pending() {
            self.properties_remote_in_change_waker.wake();
        }

        // Device is fully initialized
        *self.device_state.lock() = DeviceState::RUNNING;
        self.sse_aggregated_waker_stream.wake();

        // Main loop
        let mut device_poll_waker_receiver = self.device.poll_waker_receiver();
        let device_poll_waker = StreamOrPending::new(device_poll_waker_receiver.as_deref_mut());
        let mut device_poll_waker = device_poll_waker.fuse();

        let mut properties_remote_out_change_waker =
            self.properties_remote_out_change_waker.receiver();
        let mut properties_remote_out_change_waker =
            properties_remote_out_change_waker.by_ref().fuse();

        loop {
            // Poll
            if let Err(error) = self.device.poll(&application_driver).await.context("poll") {
                return error;
            }
            if self.device.properties().in_any_user_pending() {
                self.properties_remote_in_change_waker.wake();
            }

            // Delay or wait for poll
            let mut poll_delay = POLL_DELAY_MAX;
            if let Some(device_poll_delay) = self.device.poll_delay() {
                poll_delay = min(poll_delay, device_poll_delay);
            }
            let poll_timer = tokio::time::delay_for(poll_delay);
            let mut poll_timer = poll_timer.fuse();

            select! {
                () = poll_timer => {},
                () = device_poll_waker.select_next_some() => {},
                () = properties_remote_out_change_waker.select_next_some() => {},
            };
        }
    }
    async fn driver_run_infinite(&self) -> ! {
        *self.device_state.lock() = DeviceState::INITIALIZING;
        self.sse_aggregated_waker_stream.wake();

        loop {
            let error = self.driver_run_once().await;

            self.device.failed();
            if self.device.properties().in_any_user_pending() {
                self.properties_remote_in_change_waker.wake();
            }

            *self.device_state.lock() = DeviceState::ERROR;
            self.sse_aggregated_waker_stream.wake();
            log::warn!("device {} failed: {:?}", self.driver.address(), error);

            tokio::time::delay_for(ERROR_RESTART_DELAY).await;
        }
    }

    pub async fn run(&self) -> ! {
        let driver_runner = self.driver_run_infinite();
        pin_mut!(driver_runner);
        let mut driver_runner = driver_runner.fuse();

        let device_runner = self.device.run();
        pin_mut!(device_runner);
        let mut device_runner = device_runner.fuse();

        select! {
            _ = driver_runner => panic!("driver_runner yielded"),
            _ = device_runner => panic!("device_runner yielded"),
        }
    }
    pub async fn finalize(&self) {
        let device_state = *self.device_state.lock();
        if device_state == DeviceState::INITIALIZING || device_state == DeviceState::RUNNING {
            let application_driver = ApplicationDriver::new(&self.driver);

            let _ = self
                .device
                .deinitialize(&application_driver)
                .await
                .context("deinitialize");
            if self.device.properties().in_any_user_pending() {
                self.properties_remote_in_change_waker.wake();
            }

            *self.device_state.lock() = DeviceState::INITIALIZING;
            self.sse_aggregated_waker_stream.wake();
        }

        self.device.finalize().await;
    }
}
impl<'m, D: Device> uri_cursor::Handler for Runner<'m, D> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::GET => {
                    let device_state = *self.device_state.lock();
                    async move {
                        let response = json! {{
                            "device_state": device_state
                        }};
                        web::Response::ok_json(response)
                    }
                    .boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            uri_cursor::UriCursor::Next("properties", uri_cursor) => {
                let properties_by_name = self.device.properties().by_name();

                match &**uri_cursor {
                    uri_cursor::UriCursor::Terminal => match *request.method() {
                        http::Method::GET => {
                            let property_names =
                                properties_by_name.keys().copied().collect::<Vec<_>>();
                            async move { web::Response::ok_json(property_names) }.boxed()
                        }
                        _ => async move { web::Response::error_405() }.boxed(),
                    },
                    uri_cursor::UriCursor::Next(property_name, uri_cursor) => {
                        match properties_by_name.get(property_name) {
                            Some(property) => property.handle(request, uri_cursor),
                            None => async move { web::Response::error_404() }.boxed(),
                        }
                    }
                }
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
impl<'m, D: Device> sse_aggregated::NodeProvider for Runner<'m, D> {
    fn node(&self) -> sse_aggregated::Node {
        let properties = sse_aggregated::Node {
            terminal: None,
            children: self
                .device
                .properties()
                .by_name()
                .into_iter()
                .map(|(name, property)| {
                    (
                        sse_aggregated::PathItem::String(name.to_owned()),
                        property.node(),
                    )
                })
                .collect(),
        };

        sse_aggregated::Node {
            terminal: Some(self.sse_aggregated_waker_stream.receiver_factory()),
            children: hashmap! {
                sse_aggregated::PathItem::String("properties".to_owned()) => properties,
            },
        }
    }
}
