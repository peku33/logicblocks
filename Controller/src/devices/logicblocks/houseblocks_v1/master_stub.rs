// Stub for development on non-linux systems

use super::common::{Address, Payload};
use failure::Error;
use std::time::Duration;

#[derive(Debug)]
pub struct Master {}
impl Master {
    pub async fn transaction_out(
        &mut self,

        _service_mode: bool,
        _address: Address,
        _out_payload: Payload,
    ) -> Result<(), Error> {
        unimplemented!();
    }

    pub async fn transaction_out_in(
        &mut self,

        _service_mode: bool,
        _address: Address,
        _out_payload: Payload,
        _in_timeout: Duration,
    ) -> Result<Payload, Error> {
        unimplemented!();
    }
}
