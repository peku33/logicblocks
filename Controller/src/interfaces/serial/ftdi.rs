#[cfg(target_os = "linux")]
pub use super::ftdi_linux::*;

#[cfg(not(target_os = "linux"))]
pub use super::ftdi_stub::*;

use super::Configuration;
use crate::util::anyhow_multiple_error::AnyhowMultipleError;
use anyhow::{Context, Error, anyhow, bail};
use itertools::Itertools;
use std::{ffi, fmt, thread, time::Duration};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Descriptor {
    pub vid: u16,
    pub pid: u16,
    pub serial_number: ffi::CString,
}
impl fmt::Display for Descriptor {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{:04X}:{:04X}:{}",
            self.vid,
            self.pid,
            self.serial_number.to_string_lossy()
        )
    }
}

#[derive(Debug)]
pub struct Descriptors {
    inner: Box<[Descriptor]>,
}
impl Descriptors {
    pub fn new(inner: Box<[Descriptor]>) -> Self {
        Self { inner }
    }

    pub fn all(&self) -> &[Descriptor] {
        &self.inner
    }

    pub fn descriptor_by_serial(
        &self,
        serial: &str,
    ) -> Option<&Descriptor> {
        let descriptor = self
            .inner
            .iter()
            .filter(|descriptor| descriptor.serial_number.to_str().unwrap() == serial)
            .at_most_one()
            .unwrap()?;
        Some(descriptor)
    }

    pub fn descriptor_by_serial_or_error(
        &self,
        serial: &str,
    ) -> Result<&Descriptor, Error> {
        let descriptor = self
            .descriptor_by_serial(serial)
            .ok_or_else(|| anyhow!("descriptor {serial} is missing"))?;
        Ok(descriptor)
    }
}

#[derive(Debug)]
pub struct DeviceConfiguration {
    pub latency_timer_ms: u8,
}

#[derive(Debug)]
pub struct DeviceFailSafe {
    descriptor: Descriptor,
    configuration: Configuration,
    device_configuration: DeviceConfiguration,

    retry_count: usize,
    retry_interval: Duration,

    device: Option<Device>,
}
impl DeviceFailSafe {
    pub fn new(
        descriptor: Descriptor,
        configuration: Configuration,
        device_configuration: DeviceConfiguration,

        retry_count: usize,
        retry_interval: Duration,
    ) -> Self {
        Self {
            descriptor,
            configuration,
            device_configuration,

            retry_count,
            retry_interval,

            device: None,
        }
    }

    fn device_get(&mut self) -> Result<&mut Device, Error> {
        if self.device.is_none() {
            let device = Device::new(
                &self.descriptor,
                &self.configuration,
                &self.device_configuration,
            )
            .context("device")?;

            self.device.replace(device);
        }
        Ok(self.device.as_mut().unwrap())
    }
    fn device_release(&mut self) {
        self.device = None;
    }

    pub fn purge(&mut self) -> Result<(), Error> {
        let mut errors = Vec::<Error>::new();
        for retry_id in 0..self.retry_count {
            match try {
                let device = self.device_get().context("device_get")?;
                device.purge().context("purge")?;
            } {
                Ok(()) => return Ok(()),
                Err(error) => {
                    log::warn!("error {}/{}: {:?}", retry_id, self.retry_count, error);
                    errors.push(error);
                    self.device_release();
                    thread::sleep(self.retry_interval);
                }
            }
        }
        bail!(errors.into_iter().collect::<AnyhowMultipleError>())
    }
    pub fn write(
        &mut self,
        data: &[u8],
    ) -> Result<(), Error> {
        let mut errors = Vec::<Error>::new();
        for retry_id in 0..self.retry_count {
            match try {
                let device = self.device_get().context("device_get")?;
                device.write(data).context("write")?;
            } {
                Ok(()) => return Ok(()),
                Err(error) => {
                    log::warn!("error {}/{}: {:?}", retry_id, self.retry_count, error);
                    errors.push(error);
                    self.device_release();
                    thread::sleep(self.retry_interval);
                }
            }
        }
        bail!(errors.into_iter().collect::<AnyhowMultipleError>())
    }
    pub fn read(&mut self) -> Result<Box<[u8]>, Error> {
        let mut errors = Vec::<Error>::new();
        for retry_id in 0..self.retry_count {
            match try {
                let device = self.device_get().context("device_get")?;
                let result = device.read().context("read")?;
                result
            } {
                Ok(result) => return Ok(result),
                Err(error) => {
                    log::warn!("error {}/{}: {:?}", retry_id, self.retry_count, error);
                    errors.push(error);
                    self.device_release();
                    thread::sleep(self.retry_interval);
                }
            }
        }
        bail!(errors.into_iter().collect::<AnyhowMultipleError>())
    }
}
