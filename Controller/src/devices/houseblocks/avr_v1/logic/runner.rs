use super::super::{
    super::houseblocks_v1::{common::AddressSerial, master::Master},
    hardware::runner,
};
use crate::{
    devices::{self, GuiSummaryProvider},
    signals,
    util::{
        async_ext::{
            optional::StreamOrPending, stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        },
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
};
use async_trait::async_trait;
use futures::{future::FutureExt, join, stream::StreamExt};
use ouroboros::self_referencing;
use serde::Serialize;
use std::{borrow::Cow, fmt, mem};

pub trait DeviceFactory: Sync + Send + fmt::Debug + 'static {
    type Device<'h>: Device;

    fn new<'h>(
        hardware_device: &'h <Self::Device<'h> as Device>::HardwareDevice
    ) -> Self::Device<'h>;
}

pub trait Device: signals::Device + Sync + Send + fmt::Debug {
    type HardwareDevice: runner::Device;

    fn class() -> &'static str;

    fn as_runnable(&self) -> &dyn Runnable;
    fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
        None
    }
}

#[self_referencing]
#[derive(Debug)]
struct RunnerInner<'m, DF>
where
    DF: DeviceFactory,
{
    hardware_runner:
        runner::Runner<'m, <<DF as DeviceFactory>::Device<'m> as Device>::HardwareDevice>,

    #[borrows(hardware_runner)]
    #[not_covariant]
    device: <DF as DeviceFactory>::Device<'this>,
}

#[derive(Debug)]
pub struct Runner<'m, DF>
where
    DF: DeviceFactory,
{
    inner: RunnerInner<'m, DF>,

    gui_summary_waker: waker_stream::mpmc::Sender,
}
impl<'m, DF> Runner<'m, DF>
where
    DF: DeviceFactory,
{
    pub fn new(
        master: &'m Master,
        address_serial: AddressSerial,
        hardware_device: <<DF as DeviceFactory>::Device<'m> as Device>::HardwareDevice,
    ) -> Self {
        let hardware_runner = runner::Runner::new(master, address_serial, hardware_device);

        let inner = RunnerInner::new(hardware_runner, |hardware_runner| {
            // this should be safe as we narrow down the lifetime

            let device_static = unsafe {
                mem::transmute::<
                    &'_ <<DF as DeviceFactory>::Device<'m> as Device>::HardwareDevice,
                    &'_ <<DF as DeviceFactory>::Device<'_> as Device>::HardwareDevice,
                >(hardware_runner.device())
            };

            DF::new(device_static)
        });

        Self {
            inner,
            gui_summary_waker: waker_stream::mpmc::Sender::new(),
        }
    }

    fn device<'s>(&'s self) -> &<DF as DeviceFactory>::Device<'s> {
        // this should be safe, as we narrow down the lifetime
        self.inner.with_device(|device| unsafe {
            mem::transmute::<
                &'_ <DF as DeviceFactory>::Device<'_>,
                &'s <DF as DeviceFactory>::Device<'s>,
            >(device)
        })
    }

    fn hardware_runner(
        &self
    ) -> &runner::Runner<'m, <<DF as DeviceFactory>::Device<'m> as Device>::HardwareDevice> {
        self.inner.borrow_hardware_runner()
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // TODO: revise sequential exit of this futures
        let hardware_runner_runner = self.hardware_runner().run(exit_flag.clone());

        let device_runner = self.device().as_runnable().run(exit_flag.clone());

        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let hardware_runner_gui_summary_forwarder = self
            .hardware_runner()
            .waker() // this is from GuiSummaryProvider
            .receiver()
            .stream_take_until_exhausted(exit_flag.clone())
            .for_each(async move |()| {
                self.gui_summary_waker.wake();
            })
            .boxed();

        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let device_gui_summary_forwarder = StreamOrPending::new(
            self.device()
                .as_gui_summary_provider()
                .map(|gui_summary_provider| gui_summary_provider.waker().receiver()),
        )
        .stream_take_until_exhausted(exit_flag.clone())
        .for_each(async move |()| {
            self.gui_summary_waker.wake();
        })
        .boxed();

        let _: (Exited, Exited, (), ()) = join!(
            hardware_runner_runner,
            device_runner,
            hardware_runner_gui_summary_forwarder,
            device_gui_summary_forwarder,
        );

        Exited
    }

    pub fn finalize(self) -> <<DF as DeviceFactory>::Device<'m> as Device>::HardwareDevice {
        let inner_heads = self.inner.into_heads();
        inner_heads.hardware_runner.finalize()
    }
}

impl<'m, DF> devices::Device for Runner<'m, DF>
where
    DF: DeviceFactory,
{
    fn class(&self) -> Cow<'static, str> {
        let class = format!(
            "houseblocks/avr_v1/{}",
            <DF as DeviceFactory>::Device::class()
        );
        Cow::from(class)
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
        Some(self)
    }
}

#[async_trait]
impl<'m, DF> Runnable for Runner<'m, DF>
where
    DF: DeviceFactory,
{
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

impl<'m, DF> signals::Device for Runner<'m, DF>
where
    DF: DeviceFactory,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        self.device().targets_changed_waker()
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        self.device().sources_changed_waker()
    }

    // TODO: WTF is happening with the type in here?
    // SAFE: The signal identifier itself is 'static, so there shouldn't be problems in here
    type Identifier = <<DF as DeviceFactory>::Device<'static> as signals::Device>::Identifier;
    fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
        let device_static = unsafe {
            mem::transmute::<
                &'_ <DF as DeviceFactory>::Device<'_>,
                &'_ <DF as DeviceFactory>::Device<'static>,
            >(self.device())
        };

        device_static.by_identifier()
    }
}

#[derive(Serialize)]
struct GuiSummary {
    device: Box<dyn devices::GuiSummary>,
    hardware_runner: Box<dyn devices::GuiSummary>,
}
impl<'m, DF> devices::GuiSummaryProvider for Runner<'m, DF>
where
    DF: DeviceFactory,
{
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        let gui_summary = GuiSummary {
            device: match self.device().as_gui_summary_provider() {
                Some(gui_summary_provider) => gui_summary_provider.value(),
                None => Box::new(()),
            },
            hardware_runner: self.hardware_runner().value(),
        };
        let gui_summary = Box::new(gui_summary);
        gui_summary
    }

    fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
