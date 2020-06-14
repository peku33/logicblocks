use super::{
    super::super::houseblocks_v1::{
        common::{Address, AddressDeviceType, AddressSerial},
        master::Master,
    },
    driver::{ApplicationDriver, Driver},
};
use crate::util::waker_stream;
use async_trait::async_trait;
use failure::Error;
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use parking_lot::Mutex;
use std::{cmp::min, fmt, time::Duration};

pub trait RunContext: Sync {
    fn poll_request(&self);
}

#[async_trait]
pub trait BusDevice {
    async fn initialize(
        &self,
        driver: &dyn ApplicationDriver,
    ) -> Result<(), Error>;

    fn poll_delay(&self) -> Option<Duration>;
    async fn poll(
        &self,
        driver: &dyn ApplicationDriver,
    ) -> Result<(), Error>;

    async fn deinitialize(
        &self,
        driver: &dyn ApplicationDriver,
    ) -> Result<(), Error>;

    fn failed(&self);
}

#[async_trait]
pub trait Device: BusDevice + Sync + Send {
    type RemoteProperties<'d>: Sync + Send;

    fn new() -> Self;

    fn device_type_name() -> &'static str;
    fn address_device_type() -> AddressDeviceType;

    fn remote_properties(&self) -> Self::RemoteProperties<'_>;

    async fn run(
        &self,
        run_context: &dyn RunContext,
    ) -> !;
    async fn finalize(self);
}

const POLL_DELAY_MAXIMUM: Duration = Duration::from_secs(5);
const ERROR_DELAY: Duration = Duration::from_secs(10);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DeviceState {
    ERROR,
    INITIALIZING,
    RUNNING,
}

pub struct Runner<'m, D: Device> {
    driver: Driver<'m>,
    device: D,
    device_state: Mutex<DeviceState>,
    poll_waker: waker_stream::Sender,
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
        let poll_waker = waker_stream::Sender::new();

        Self {
            driver,
            device,
            device_state,
            poll_waker,
        }
    }

    pub fn remote_properties(&self) -> D::RemoteProperties<'_> {
        self.device.remote_properties()
    }
    pub fn device_state(&self) -> DeviceState {
        *self.device_state.lock()
    }

    async fn driver_run_once(&self) -> Error {
        *self.device_state.lock() = DeviceState::INITIALIZING;
        log::trace!("{}: driver_run_once - initializing", self);

        // Hardware initializing & avr_v1
        log::trace!("{}: driver_run_once - driver - preparing", self);
        match self.driver.prepare().await {
            Ok(()) => {
                log::trace!("{}: driver_run_once - driver - preparing ok", self,);
            }
            Err(error) => {
                log::warn!(
                    "{}: driver_run_once - driver - preparing error: {:?}",
                    self,
                    error
                );
                return error;
            }
        };

        // User mode initializing
        log::trace!("{}: driver_run_once - device - initializing", self);
        match self.device.initialize(&self.driver).await {
            Ok(()) => {
                log::trace!("{}: driver_run_once - device - initializing ok", self,);
            }
            Err(error) => {
                log::warn!(
                    "{}: driver_run_once - device - initializing error: {:?}",
                    self,
                    error
                );
                return error;
            }
        };

        // Main running loop
        *self.device_state.lock() = DeviceState::RUNNING;
        log::debug!("{}: driver_run_once - initialized", self);

        let mut poll_waker = self.poll_waker.receiver();
        loop {
            // Poll
            log::trace!("{}: driver_run_once - device - poll", self);
            match self.device.poll(&self.driver).await {
                Ok(()) => {
                    log::trace!("{}: driver_run_once - device - poll ok", self,);
                }
                Err(error) => {
                    log::warn!(
                        "{}: driver_run_once - device - poll error: {:?}",
                        self,
                        error
                    );
                    return error;
                }
            };

            // Poll delay
            let mut poll_delay = POLL_DELAY_MAXIMUM;
            if let Some(device_poll_delay) = self.device.poll_delay() {
                poll_delay = min(poll_delay, device_poll_delay);
            }

            log::trace!(
                "{}: driver_run_once - device - poll delay for: {:?}",
                self,
                poll_delay
            );
            let poll_timer = tokio::time::delay_for(poll_delay);
            let mut poll_timer = poll_timer.fuse();
            select! {
                () = poll_timer => {
                    log::trace!("driver_run_once - waked by delay");
                },
                () = poll_waker.select_next_some() => {
                    log::trace!("driver_run_once - waked by poll_waker");
                },
            }
            log::trace!("{}: driver_run_once - device - poll delay completed", self);
        }
    }
    async fn driver_run_infinite(&self) -> ! {
        log::trace!("{}: driver_run_infinite - starting", self);
        loop {
            log::trace!("{}: driver_run_infinite - driver_run_once starting", self);
            let error = self.driver_run_once().await;
            *self.device_state.lock() = DeviceState::ERROR;
            self.device.failed();
            log::warn!("{}: driver_run_infinite - error: {:?}", self, error);

            log::trace!("{}: driver_run_infinite - error delay starting", self);
            tokio::time::delay_for(ERROR_DELAY).await;
            log::trace!("{}: driver_run_infinite - error delay complete", self);
        }
    }

    pub async fn run(&self) -> ! {
        log::trace!("{}: run - starting", self);

        let driver_runner = self.driver_run_infinite();
        pin_mut!(driver_runner);
        let mut driver_runner = driver_runner.fuse();

        let device_runner = self.device.run(self);
        pin_mut!(device_runner);
        let mut device_runner = device_runner.fuse();

        select! {
            _ = driver_runner => panic!("driver_runner yielded"),
            _ = device_runner => panic!("device_runner yielded"),
        }
    }

    pub async fn finalize(self) {
        let self_string = format!("{}", self);
        log::trace!("{}: finalize - starting", self_string);

        let device_state = *self.device_state.lock();
        match device_state {
            DeviceState::RUNNING | DeviceState::INITIALIZING => {
                log::trace!(
                    "{}: finalize - device is running, deinitializing",
                    self_string
                );
                match self.device.deinitialize(&self.driver).await {
                    Ok(()) => {
                        *self.device_state.lock() = DeviceState::INITIALIZING;
                        log::trace!("{}: finalize - device deinitializing - ok", self_string);
                    }
                    Err(error) => {
                        *self.device_state.lock() = DeviceState::ERROR;
                        log::warn!(
                            "{}: finalize - device deinitializing - error: {:?}",
                            self_string,
                            error
                        );
                    }
                }
            }
            DeviceState::ERROR => {
                log::trace!(
                    "{}: finalize - device not initialized, not deinitializing",
                    self_string
                );
            }
        };
        log::trace!("{}: finalize - finalizing device", self_string);
        self.device.finalize().await;

        log::trace!("{}: finalize - completed", self_string);
    }
}
impl<'m, D: Device> RunContext for Runner<'m, D> {
    fn poll_request(&self) {
        log::trace!("{}: poll_request called", self);
        self.poll_waker.wake();
    }
}
impl<'m, D: Device> fmt::Display for Runner<'m, D> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{} ({:?})", self.driver, self.device_state())
    }
}
