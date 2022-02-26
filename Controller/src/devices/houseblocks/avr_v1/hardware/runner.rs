use super::{
    super::super::houseblocks_v1::{
        common::{Address, AddressDeviceType, AddressSerial},
        master::Master,
    },
    driver::{ApplicationDriver, Driver},
};
use crate::{
    devices,
    util::{
        async_ext::optional::StreamOrPending,
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use futures::{
    future::{Either, FutureExt},
    join, select,
    stream::StreamExt,
};
use parking_lot::Mutex;
use serde::Serialize;
use std::{cmp::min, fmt, time::Duration};

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
    fn user_pending(&self) -> bool;
    fn device_reset(&self);

    type Remote: Sync + Send;
    fn remote(&self) -> Self::Remote;
}

pub trait Device: BusDevice + Sync + Send + Sized + fmt::Debug {
    fn device_type_name() -> &'static str;
    fn address_device_type() -> AddressDeviceType;

    fn as_runnable(&self) -> Option<&dyn Runnable> {
        None
    }

    type Properties: Properties;
    fn properties(&self) -> &Self::Properties;

    fn poll_waker_receiver(&self) -> Option<waker_stream::mpsc::ReceiverLease> {
        None
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum DeviceState {
    Error,
    Initializing,
    Running,
}

#[derive(Debug)]
pub struct Runner<'m, D: Device> {
    driver: Driver<'m>,
    device: D,

    device_state: Mutex<DeviceState>,

    properties_remote_in_change_waker: waker_stream::mpsc::SenderReceiver,
    properties_remote_out_change_waker: waker_stream::mpsc::SenderReceiver,

    gui_summary_waker: waker_stream::mpmc::Sender,
}
impl<'m, D: Device> Runner<'m, D> {
    const POLL_DELAY_MAX: Duration = Duration::from_secs(5);
    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(10);

    pub fn new(
        master: &'m Master,
        device: D,
        address_serial: AddressSerial,
    ) -> Self {
        let driver = Driver::new(
            master,
            Address {
                device_type: D::address_device_type(),
                serial: address_serial,
            },
        );
        let device_state = Mutex::new(DeviceState::Initializing);

        Self {
            driver,
            device,

            device_state,

            properties_remote_in_change_waker: waker_stream::mpsc::SenderReceiver::new(),
            properties_remote_out_change_waker: waker_stream::mpsc::SenderReceiver::new(),

            gui_summary_waker: waker_stream::mpmc::Sender::new(),
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

    async fn driver_run_once(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        *self.device_state.lock() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        // Make device properties in uninitialized state
        self.device.properties().device_reset();
        if self.device.properties().user_pending() {
            self.properties_remote_in_change_waker.wake();
        }

        // Hardware initializing & avr_v1
        self.driver.prepare().await.context("initial prepare")?;

        // Device is prepared in application mode, we can start application driver
        let application_driver = ApplicationDriver::new(&self.driver);

        // User mode initializing
        self.device
            .initialize(&application_driver)
            .await
            .context("initialize")?;
        if self.device.properties().user_pending() {
            self.properties_remote_in_change_waker.wake();
        }

        // Device is fully initialized
        *self.device_state.lock() = DeviceState::Running;
        self.gui_summary_waker.wake();

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
            self.device
                .poll(&application_driver)
                .await
                .context("poll")?;
            if self.device.properties().user_pending() {
                self.properties_remote_in_change_waker.wake();
            }

            // Delay or wait for poll
            let mut poll_delay = Self::POLL_DELAY_MAX;
            if let Some(device_poll_delay) = self.device.poll_delay() {
                poll_delay = min(poll_delay, device_poll_delay);
            }

            select! {
                () = tokio::time::sleep(poll_delay).fuse() => {},
                () = device_poll_waker.select_next_some() => {},
                () = properties_remote_out_change_waker.select_next_some() => {},
                () = exit_flag => break,
            };
        }

        // Finalize
        self.device
            .deinitialize(&application_driver)
            .await
            .context("deinitialize")?;
        self.device.properties().device_reset();
        if self.device.properties().user_pending() {
            self.properties_remote_in_change_waker.wake();
        }

        // TODO: reset device to reinitialize state?

        *self.device_state.lock() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        Ok(Exited)
    }
    async fn driver_run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        loop {
            let error = match self
                .driver_run_once(exit_flag.clone())
                .await
                .context("driver_run_once")
            {
                Ok(Exited) => break,
                Err(error) => error,
            };
            log::error!("device {} failed: {:?}", self.driver.address(), error);

            // Make device properties in uninitialized state
            self.device.properties().device_reset();
            self.device.failed();
            if self.device.properties().user_pending() {
                self.properties_remote_in_change_waker.wake();
            }

            *self.device_state.lock() = DeviceState::Error;
            self.gui_summary_waker.wake();

            select! {
                () = tokio::time::sleep(Self::ERROR_RESTART_DELAY).fuse() => {},
                () = exit_flag => break,
            }
        }

        Exited
    }

    pub async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // TODO: check whether this should be exited sequentially
        let driver_runner = self.driver_run(exit_flag.clone());

        let device_exit_flag = exit_flag.clone();
        let device_runner = match self.device.as_runnable() {
            Some(runnable) => Either::Left(runnable.run(device_exit_flag)),
            None => Either::Right(device_exit_flag.map(|()| Exited)),
        };

        let _: (Exited, Exited) = join!(driver_runner, device_runner);

        Exited
    }

    pub fn finalize(self) -> D {
        self.device
    }
}

#[derive(Serialize)]
struct GuiSummary {
    device_state: DeviceState,
}
impl<'m, D: Device> devices::GuiSummaryProvider for Runner<'m, D> {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        let gui_summary = GuiSummary {
            device_state: *self.device_state.lock(),
        };
        let gui_summary = Box::new(gui_summary);
        gui_summary
    }

    fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
