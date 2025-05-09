use super::{
    Configuration,
    ftdi::{Descriptor, Descriptors, DeviceConfiguration},
};
use anyhow::Error;

#[derive(Debug)]
pub struct Global {}
impl Global {
    pub fn new() -> Result<Self, Error> {
        unimplemented!();
    }
    pub fn find_descriptors(&mut self) -> Result<Descriptors, Error> {
        unimplemented!();
    }
}

#[derive(Debug)]
pub struct Device {}
impl Device {
    pub fn new(
        _descriptor: &Descriptor,
        _configuration: &Configuration,
        _device_configuration: &DeviceConfiguration,
    ) -> Result<Self, Error> {
        unimplemented!();
    }

    pub fn purge(&mut self) -> Result<(), Error> {
        unimplemented!();
    }
    pub fn write(
        &mut self,
        _data: &[u8],
    ) -> Result<(), Error> {
        unimplemented!();
    }
    pub fn read(&mut self) -> Result<Box<[u8]>, Error> {
        unimplemented!();
    }
}
