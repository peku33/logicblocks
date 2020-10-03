// Stub for development on non-linux systems

use super::{
    common::{Address, Payload},
    master::MasterDescriptor,
};
use anyhow::Error;
use std::{fmt, time::Duration};

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
impl fmt::Display for Master {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        return write!(f, "Master(Stub)");
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
