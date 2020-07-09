use super::{
    super::houseblocks_v1::{common::AddressSerial, master::Master},
    hardware::runner,
};
use crate::{
    logic::device,
    web::{
        sse_aggregated::{Node, NodeProvider, PathItem},
        uri_cursor::{Handler, UriCursor},
        Request, Response,
    },
};
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    pin_mut, select,
};
use http::Method;
use maplit::hashmap;
use serde_json::json;
use std::borrow::Cow;

#[async_trait]
pub trait Device: Sync + Send + Handler + NodeProvider {
    type HardwareDevice: runner::Device;

    fn new() -> Self;
    fn class() -> &'static str;

    fn signals(&self) -> device::Signals;

    async fn run(
        &self,
        remote_properties: <Self::HardwareDevice as runner::Device>::RemoteProperties<'_>,
    ) -> !;
    async fn finalize(self);
}

pub struct Runner<'m, D: Device> {
    device: D,
    hardware_runner: runner::Runner<'m, D::HardwareDevice>,
}
impl<'m, D: Device> Runner<'m, D> {
    pub fn new(
        master: &'m Master,
        address_serial: AddressSerial,
    ) -> Self {
        let device = D::new();
        let hardware_runner = runner::Runner::new(master, address_serial);

        Self {
            device,
            hardware_runner,
        }
    }
}
#[async_trait]
impl<'m, D: Device> device::Device for Runner<'_, D> {
    fn class(&self) -> Cow<'static, str> {
        let class = format!("houseblocks/avr_v1/{}", D::class());
        Cow::from(class)
    }

    fn signals(&self) -> device::Signals {
        self.device.signals()
    }

    async fn run(&self) -> ! {
        let hardware_runner_runner = self.hardware_runner.run();
        pin_mut!(hardware_runner_runner);
        let mut hardware_runner_runner = hardware_runner_runner.fuse();

        let remote_properties = self.hardware_runner.remote_properties();

        let device_runner = self.device.run(remote_properties);
        pin_mut!(device_runner);
        let mut device_runner = device_runner.fuse();

        select! {
            _ = hardware_runner_runner => panic!("hardware_runner_runner yielded"),
            _ = device_runner => panic!("device_runner yielded"),
        }
    }
    async fn finalize(self: Box<Self>) {
        self.device.finalize().await;
        self.hardware_runner.finalize().await;
    }
}
impl<'m, D: Device> Handler for Runner<'m, D> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            UriCursor::Terminal => match *request.method() {
                Method::GET => {
                    let response = json!({
                        "device_state": self.hardware_runner.device_state(),
                    });
                    async move { Response::ok_json(response) }.boxed()
                }
                _ => async move { Response::error_405() }.boxed(),
            },
            UriCursor::Next("device", uri_cursor) => self.device.handle(request, uri_cursor),
            _ => async move { Response::error_404() }.boxed(),
        }
    }
}
impl<'m, D: Device> NodeProvider for Runner<'m, D> {
    fn node(&self) -> Node {
        Node::Children(hashmap! {
            PathItem::String("device".to_owned()) => self.device.node(),
        })
    }
}