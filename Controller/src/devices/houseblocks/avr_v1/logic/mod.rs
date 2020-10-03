use super::{
    super::houseblocks_v1::{common::AddressSerial, master::Master},
    hardware::runner,
};
use crate::{
    devices, signals,
    util::waker_stream,
    web::{self, sse_aggregated, uri_cursor},
};
use async_trait::async_trait;
use futures::{
    future::{pending, BoxFuture},
    pin_mut, select, FutureExt, StreamExt,
};
use maplit::hashmap;
use std::{borrow::Cow, fmt};

#[async_trait]
pub trait Device: Sync + Send + fmt::Debug {
    type HardwareDevice: runner::Device;

    fn new(
        properties_remote: <<Self::HardwareDevice as runner::Device>::Properties as runner::Properties>::Remote
    ) -> Self;

    fn class() -> &'static str;

    fn as_signals_device(&self) -> Option<&dyn signals::Device>;
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler>;
    fn as_sse_aggregated_node_provider(&self) -> Option<&dyn sse_aggregated::NodeProvider>;

    fn properties_remote_in_changed(&self);
    fn properties_remote_out_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease;

    async fn run(&self) -> ! {
        pending().await
    }
    async fn finalize(&self) {}
}

#[derive(Debug)]
pub struct Runner<'m, D: Device> {
    // device: D must declared used before hardware_runner to keep destruction order (device first)
    device: D,
    hardware_runner: runner::Runner<'m, D::HardwareDevice>,
}
impl<'m, D: Device> Runner<'m, D> {
    pub fn new(
        master: &'m Master,
        address_serial: AddressSerial,
    ) -> Self {
        let hardware_runner = runner::Runner::new(master, address_serial);
        let properties_remote = hardware_runner.properties_remote();

        let device = D::new(properties_remote);

        Self {
            hardware_runner,
            device,
        }
    }
}
#[async_trait]
impl<'m, D: Device> devices::Device for Runner<'m, D> {
    fn class(&self) -> Cow<'static, str> {
        let class = format!("houseblocks/avr_v1/{}", D::class());
        Cow::from(class)
    }

    fn as_signals_device(&self) -> Option<&dyn signals::Device> {
        self.device.as_signals_device()
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        Some(self)
    }
    fn as_sse_aggregated_node_provider(&self) -> Option<&dyn sse_aggregated::NodeProvider> {
        Some(self)
    }

    async fn run(&self) -> ! {
        let hardware_runner_run = self.hardware_runner.run();
        pin_mut!(hardware_runner_run);
        let mut hardware_runner_run = hardware_runner_run.fuse();

        let device_run = self.device.run();
        let mut device_run = device_run.fuse();

        let mut properties_remote_in_changed_waker = self
            .hardware_runner
            .properties_remote_in_change_waker_receiver();
        let properties_remote_in_changed_waker_forwarder = properties_remote_in_changed_waker
            .by_ref()
            .for_each(async move |()| {
                self.device.properties_remote_in_changed();
            });
        pin_mut!(properties_remote_in_changed_waker_forwarder);
        let mut properties_remote_in_changed_waker_forwarder =
            properties_remote_in_changed_waker_forwarder.fuse();

        let mut properties_remote_out_changed_waker =
            self.device.properties_remote_out_changed_waker_receiver();
        let properties_remote_out_changed_waker_forwarder = properties_remote_out_changed_waker
            .by_ref()
            .for_each(async move |()| {
                self.hardware_runner
                    .properties_remote_out_change_waker_wake();
            });
        pin_mut!(properties_remote_out_changed_waker_forwarder);
        let mut properties_remote_out_changed_waker_forwarder =
            properties_remote_out_changed_waker_forwarder.fuse();

        select! {
            _ = hardware_runner_run => panic!("hardware_runner_run yielded"),
            _ = device_run => panic!("device_run yielded"),
            _ = properties_remote_in_changed_waker_forwarder => panic!("properties_remote_in_changed_waker_forwarder yielded"),
            _ = properties_remote_out_changed_waker_forwarder => panic!("properties_remote_out_changed_waker_forwarder yielded"),
        }
    }
    async fn finalize(&self) {
        self.device.finalize().await;
        self.hardware_runner.finalize().await;
    }
}
impl<'m, D: Device> uri_cursor::Handler for Runner<'m, D> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("device", uri_cursor) => {
                match self.device.as_web_handler() {
                    Some(device_web_handler) => device_web_handler.handle(request, uri_cursor),
                    None => async move { web::Response::error_404() }.boxed(),
                }
            }
            uri_cursor::UriCursor::Next("hardware_runner", uri_cursor) => {
                self.hardware_runner.handle(request, uri_cursor)
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
impl<'m, D: Device> sse_aggregated::NodeProvider for Runner<'m, D> {
    fn node(&self) -> sse_aggregated::Node {
        let mut children = hashmap! {
            sse_aggregated::PathItem::String("hardware_runner".to_owned()) => self.hardware_runner.node(),
        };
        if let Some(device_sse_aggregated_node_provider) =
            self.device.as_sse_aggregated_node_provider()
        {
            children.insert(
                sse_aggregated::PathItem::String("device".to_owned()),
                device_sse_aggregated_node_provider.node(),
            );
        }

        sse_aggregated::Node {
            terminal: None,
            children,
        }
    }
}
