use super::{
    super::houseblocks_v1::{common::AddressSerial, master::Master},
    hardware::runner,
};
use crate::{
    devices::{self, GuiSummaryProvider},
    signals,
    util::{
        scoped_async::{ExitFlag, Exited, Runnable},
        waker_stream,
    },
    web::uri_cursor,
};
use async_trait::async_trait;
use futures::{
    future::{join, FutureExt, OptionFuture},
    pin_mut, select,
    stream::StreamExt,
};
use serde::Serialize;
use std::{borrow::Cow, fmt};

pub trait Device: Sync + Send + fmt::Debug {
    type HardwareDevice: runner::Device;

    fn new(
        properties_remote: <<Self::HardwareDevice as runner::Device>::Properties as runner::Properties>::Remote
    ) -> Self;

    fn class() -> &'static str;

    fn as_runnable(&self) -> Option<&dyn Runnable> {
        None
    }
    fn as_signals_device(&self) -> &dyn signals::Device;
    fn as_gui_summary_provider(&self) -> &dyn GuiSummaryProvider;
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        None
    }

    fn properties_remote_in_changed(&self);
    fn properties_remote_out_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease;
}

#[derive(Debug)]
pub struct Runner<'m, D: Device> {
    // device: D must declared used before hardware_runner to keep destruction order (device first)
    device: D,
    hardware_runner: runner::Runner<'m, D::HardwareDevice>,

    gui_summary_waker: waker_stream::mpmc::Sender,
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

            gui_summary_waker: waker_stream::mpmc::Sender::new(),
        }
    }

    pub async fn run(
        &self,
        exit_flag: ExitFlag,
    ) -> Exited {
        let hardware_runner_run = self.hardware_runner.run(exit_flag.clone());

        let device_run = OptionFuture::from(
            self.device
                .as_runnable()
                .map(|device_runnable| device_runnable.run(exit_flag.clone())),
        );

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

        let hardware_runner_gui_summary_waker_forwarder = self
            .hardware_runner
            .get_waker()
            .receiver()
            .for_each(async move |()| {
                self.gui_summary_waker.wake();
            });
        pin_mut!(hardware_runner_gui_summary_waker_forwarder);
        let mut hardware_runner_gui_summary_waker_forwarder =
            hardware_runner_gui_summary_waker_forwarder.fuse();

        let device_gui_summary_waker_forwarder = self
            .device
            .as_gui_summary_provider()
            .get_waker()
            .receiver()
            .for_each(async move |()| {
                self.gui_summary_waker.wake();
            });
        pin_mut!(device_gui_summary_waker_forwarder);
        let mut device_gui_summary_waker_forwarder = device_gui_summary_waker_forwarder.fuse();

        select! {
            _ = join(hardware_runner_run, device_run).fuse() => {},
            _ = properties_remote_in_changed_waker_forwarder => panic!("properties_remote_in_changed_waker_forwarder yielded"),
            _ = properties_remote_out_changed_waker_forwarder => panic!("properties_remote_out_changed_waker_forwarder yielded"),
            _ = hardware_runner_gui_summary_waker_forwarder => panic!("hardware_runner_gui_summary_waker_forwarder yielded"),
            _ = device_gui_summary_waker_forwarder => panic!("device_gui_summary_waker_forwarder yielded"),
        }

        Exited
    }
}
impl<'m, D: Device> devices::Device for Runner<'m, D> {
    fn class(&self) -> Cow<'static, str> {
        let class = format!("houseblocks/avr_v1/{}", D::class());
        Cow::from(class)
    }

    fn as_runnable(&self) -> Option<&dyn Runnable> {
        Some(self)
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self.device.as_signals_device()
    }
    fn as_gui_summary_provider(&self) -> &dyn GuiSummaryProvider {
        self
    }
}
#[async_trait]
impl<'m, D: Device> Runnable for Runner<'m, D> {
    async fn run(
        &self,
        exit_flag: ExitFlag,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
#[derive(Serialize)]
struct GuiSummary {
    device: Box<dyn devices::GuiSummary>,
    hardware_runner: Box<dyn devices::GuiSummary>,
}
impl<'m, D: Device> GuiSummaryProvider for Runner<'m, D> {
    fn get_value(&self) -> Box<dyn devices::GuiSummary> {
        Box::new(GuiSummary {
            device: self.device.as_gui_summary_provider().get_value(),
            hardware_runner: self.hardware_runner.get_value(),
        })
    }

    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
