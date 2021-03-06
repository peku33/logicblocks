use super::{
    super::super::houseblocks_v1::{
        common::{Address, AddressDeviceType, AddressSerial},
        master::Master,
    },
    driver::{ApplicationDriver, Driver},
    property,
};
use crate::{
    devices,
    util::{
        optional_async::StreamOrPending,
        scoped_async::{ExitFlag, Exited, Runnable},
        waker_stream,
    },
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use futures::{
    future::{FutureExt, OptionFuture},
    join, select,
    stream::StreamExt,
};
use parking_lot::Mutex;
use serde::Serialize;
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

pub trait Device: BusDevice + Sync + Send + Sized + fmt::Debug {
    fn new() -> Self;

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

#[derive(Serialize, Copy, Clone, PartialEq, Eq, Debug)]
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
        address_serial: AddressSerial,
    ) -> Self {
        let driver = Driver::new(
            master,
            Address::new(D::address_device_type(), address_serial),
        );
        let device = D::new();

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
        mut exit_flag: ExitFlag,
    ) -> Result<Exited, Error> {
        *self.device_state.lock() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        // Hardware initializing & avr_v1
        self.driver.prepare().await.context("initial prepare")?;

        // Device is prepared in application mode, we can start application driver
        let application_driver = ApplicationDriver::new(&self.driver);

        // User mode initializing
        self.device
            .initialize(&application_driver)
            .await
            .context("initialize")?;
        if self.device.properties().in_any_user_pending() {
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
            if self.device.properties().in_any_user_pending() {
                self.properties_remote_in_change_waker.wake();
            }

            // Delay or wait for poll
            let mut poll_delay = Self::POLL_DELAY_MAX;
            if let Some(device_poll_delay) = self.device.poll_delay() {
                poll_delay = min(poll_delay, device_poll_delay);
            }
            let poll_timer = tokio::time::delay_for(poll_delay);
            let mut poll_timer = poll_timer.fuse();

            select! {
                () = poll_timer => {},
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
        if self.device.properties().in_any_user_pending() {
            self.properties_remote_in_change_waker.wake();
        }

        *self.device_state.lock() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        Ok(Exited)
    }
    async fn driver_run(
        &self,
        exit_flag: ExitFlag,
    ) -> Exited {
        // Prepare
        *self.device_state.lock() = DeviceState::Initializing;
        self.gui_summary_waker.wake();

        // Run
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

            self.device.failed();
            if self.device.properties().in_any_user_pending() {
                self.properties_remote_in_change_waker.wake();
            }

            *self.device_state.lock() = DeviceState::Error;
            self.gui_summary_waker.wake();

            tokio::time::delay_for(Self::ERROR_RESTART_DELAY).await;
        }

        Exited
    }

    pub async fn run(
        &self,
        exit_flag: ExitFlag,
    ) -> Exited {
        let driver_runner = self.driver_run(exit_flag.clone());

        let device_runner = OptionFuture::from(
            self.device
                .as_runnable()
                .map(|device_runnable| device_runnable.run(exit_flag.clone())),
        );

        join!(driver_runner, device_runner);

        Exited
    }
}

#[derive(Serialize)]
struct GuiSummary {
    device_state: DeviceState,
}
impl<'m, D: Device> devices::GuiSummaryProvider for Runner<'m, D> {
    fn get_value(&self) -> Box<dyn devices::GuiSummary> {
        Box::new(GuiSummary {
            device_state: *self.device_state.lock(),
        })
    }

    fn get_waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
