// Stub for development on non-linux systems

use super::common::{Address, Payload};
use super::master::MasterDescriptor;
use failure::Error;
use std::time::Duration;

#[derive(Debug)]
pub struct Master {}
impl Master {
    pub fn new(_master_descriptor: MasterDescriptor) -> Result<Self, Error> {
        unimplemented!();
    }

    pub async fn transaction_out(
        &self,

        _service_mode: bool,
        _address: Address,
        _out_payload: Payload,
    ) -> Result<(), Error> {
        unimplemented!();
    }
    pub async fn transaction_out_in(
        &self,

        _service_mode: bool,
        _address: Address,
        _out_payload: Payload,
        _in_timeout: Duration,
    ) -> Result<Payload, Error> {
        unimplemented!();
    }
    pub async fn transaction_device_discovery(&self) -> Result<Address, Error> {
        unimplemented!()
    }
}

pub struct MasterContext {}
impl MasterContext {
    pub fn new() -> Result<Self, Error> {
        unimplemented!();
    }
    pub fn find_master_descriptors(&self) -> Result<Vec<MasterDescriptor>, Error> {
        unimplemented!();
    }
}
