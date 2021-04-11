use super::{
    super::houseblocks_v1::{common::AddressSerial, master::Master},
    hardware::runner,
};
use crate::{
    devices::{self, GuiSummaryProvider},
    signals,
    util::{
        async_flag,
        optional_async::StreamOrPending,
        runtime::{Exited, Runnable},
        waker_stream,
    },
    web::uri_cursor,
};
use async_trait::async_trait;
use futures::{future::FutureExt, join, stream::StreamExt};
use serde::Serialize;
use std::{borrow::Cow, fmt};

pub trait Device: Runnable + signals::Device + Sync + Send + fmt::Debug {
    type HardwareDevice: runner::Device;

    fn new(
        properties_remote: <<Self::HardwareDevice as runner::Device>::Properties as runner::Properties>::Remote
    ) -> Self;

    fn class() -> &'static str;

    fn as_gui_summary_provider(&self) -> Option<&dyn GuiSummaryProvider> {
        None
    }
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
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // TODO: replace take_until with finalizing version
        // TODO: revise sequential exit of this futures
        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let hardware_runner_runner = self.hardware_runner.run(exit_flag.clone());
        let device_runner = self.device.run(exit_flag.clone());

        let mut properties_remote_in_changed_waker_receiver = self
            .hardware_runner
            .properties_remote_in_change_waker_receiver();
        let mut properties_remote_in_changed_waker_receiver =
            properties_remote_in_changed_waker_receiver
                .by_ref()
                .take_until(exit_flag.clone());
        let properties_remote_in_changed_forwarder = properties_remote_in_changed_waker_receiver
            .by_ref()
            .for_each(async move |()| {
                self.device.properties_remote_in_changed();
            })
            .boxed();

        let mut properties_remote_out_changed_waker_receiver =
            self.device.properties_remote_out_changed_waker_receiver();
        let mut properties_remote_out_changed_waker_receiver =
            properties_remote_out_changed_waker_receiver
                .by_ref()
                .take_until(exit_flag.clone());
        let properties_remote_out_changed_forwarder = properties_remote_out_changed_waker_receiver
            .by_ref()
            .for_each(async move |()| {
                self.hardware_runner
                    .properties_remote_out_change_waker_wake();
            })
            .boxed();

        let mut hardware_runner_gui_summary_waker_receiver =
            self.hardware_runner.waker().receiver();
        let mut hardware_runner_gui_summary_waker_receiver =
            hardware_runner_gui_summary_waker_receiver
                .by_ref()
                .take_until(exit_flag.clone());
        let hardware_runner_gui_summary_forwarder = hardware_runner_gui_summary_waker_receiver
            .by_ref()
            .for_each(async move |()| {
                self.gui_summary_waker.wake();
            })
            .boxed();

        let mut device_gui_summary_waker_receiver = StreamOrPending::new(
            self.device
                .as_gui_summary_provider()
                .map(|gui_summary_provider| gui_summary_provider.waker().receiver()),
        );
        let mut device_gui_summary_waker_receiver = device_gui_summary_waker_receiver
            .by_ref()
            .take_until(exit_flag.clone());
        let device_gui_summary_forwarder = device_gui_summary_waker_receiver
            .by_ref()
            .for_each(async move |()| {
                self.gui_summary_waker.wake();
            })
            .boxed();

        let _: (Exited, Exited, (), (), (), ()) = join!(
            hardware_runner_runner,
            device_runner,
            properties_remote_in_changed_forwarder,
            properties_remote_out_changed_forwarder,
            hardware_runner_gui_summary_forwarder,
            device_gui_summary_forwarder,
        );

        assert!(properties_remote_in_changed_waker_receiver.is_stopped());
        assert!(properties_remote_out_changed_waker_receiver.is_stopped());
        assert!(hardware_runner_gui_summary_waker_receiver.is_stopped());
        assert!(device_gui_summary_waker_receiver.is_stopped());

        Exited
    }
}
impl<'m, D: Device> devices::Device for Runner<'m, D> {
    fn class(&self) -> Cow<'static, str> {
        let class = format!("houseblocks/avr_v1/{}", D::class());
        Cow::from(class)
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn GuiSummaryProvider> {
        Some(self)
    }
}
#[async_trait]
impl<'m, D: Device> Runnable for Runner<'m, D> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl<'m, D: Device> signals::Device for Runner<'m, D> {
    fn signal_targets_changed_wake(&self) {
        self.device.signal_targets_changed_wake()
    }

    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.device.signal_sources_changed_waker_receiver()
    }

    fn signals(&self) -> signals::Signals {
        self.device.signals()
    }
}
#[derive(Serialize)]
struct GuiSummary {
    device: Box<dyn devices::GuiSummary>,
    hardware_runner: Box<dyn devices::GuiSummary>,
}
impl<'m, D: Device> GuiSummaryProvider for Runner<'m, D> {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        Box::new(GuiSummary {
            device: match self.device.as_gui_summary_provider() {
                Some(gui_summary_provider) => gui_summary_provider.value(),
                None => Box::new(()),
            },
            hardware_runner: self.hardware_runner.value(),
        })
    }

    fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
