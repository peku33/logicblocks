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
        async_flag, async_waker,
        runnable::{Exited, Runnable},
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use futures::{
    future::{Either, FutureExt},
    join, select,
    stream::StreamExt,
};
use parking_lot::RwLock;
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

    // the device was internally reset, so need to clear internal state
    fn reset(&self) {}
}

pub trait Device: BusDevice + Sync + Send + Sized + fmt::Debug {
    fn device_type_name() -> &'static str;
    fn address_device_type() -> AddressDeviceType;

    fn poll_waker(&self) -> Option<&async_waker::mpsc::Signal>;

    fn as_runnable(&self) -> Option<&dyn Runnable>;
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

    device_state: RwLock<DeviceState>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl<'m, D: Device> Runner<'m, D> {
    const POLL_DELAY_MAX: Duration = Duration::from_secs(5);
    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(10);

    pub fn new(
        master: &'m Master,
        address_serial: AddressSerial,
        device: D,
    ) -> Self {
        let driver = Driver::new(
            master,
            Address {
                device_type: D::address_device_type(),
                serial: address_serial,
            },
        );
        let device_state = RwLock::new(DeviceState::Initializing);

        Self {
            driver,
            device,

            device_state,

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    pub fn device(&self) -> &D {
        &self.device
    }

    async fn driver_run_once(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        *self.device_state.write() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        self.device.reset();

        // Hardware initializing & avr_v1
        self.driver.prepare().await.context("initial prepare")?;

        // Device is prepared in application mode, we can start application driver
        let application_driver = ApplicationDriver::new(&self.driver);

        // User mode initializing
        self.device
            .initialize(&application_driver)
            .await
            .context("initialize")?;

        // Device is fully initialized
        *self.device_state.write() = DeviceState::Running;
        self.gui_summary_waker.wake();

        // Main loop
        let mut device_poll_waker = StreamOrPending::new(
            self.device
                .poll_waker()
                .map(|poll_waker| poll_waker.receiver()),
        )
        .fuse();

        loop {
            // Poll
            self.device
                .poll(&application_driver)
                .await
                .context("poll")?;

            // Delay or wait for poll
            let mut poll_delay = Self::POLL_DELAY_MAX;
            if let Some(device_poll_delay) = self.device.poll_delay() {
                poll_delay = min(poll_delay, device_poll_delay);
            }

            select! {
                () = tokio::time::sleep(poll_delay).fuse() => {},
                () = device_poll_waker.select_next_some() => {},
                () = exit_flag => break,
            };
        }

        // Finalize
        self.device
            .deinitialize(&application_driver)
            .await
            .context("deinitialize")?;

        self.device.reset();

        *self.device_state.write() = DeviceState::Initializing;
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

            self.device.reset();

            *self.device_state.write() = DeviceState::Error;
            self.gui_summary_waker.wake();

            select! {
                () = tokio::time::sleep(Self::ERROR_RESTART_DELAY).fuse() => {},
                () = exit_flag => break,
            }
        }

        Exited
    }

    async fn run(
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

#[async_trait]
impl<D: Device> Runnable for Runner<'_, D> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[derive(Debug, Serialize)]
pub struct GuiSummary {
    device_state: DeviceState,
}
impl<D: Device> devices::gui_summary::Device for Runner<'_, D> {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let device_state = *self.device_state.read();

        Self::Value { device_state }
    }
}
