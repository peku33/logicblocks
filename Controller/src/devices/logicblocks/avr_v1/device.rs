#![cfg(target_os = "linux")]

use super::super::houseblocks_v1::common::{Address, Payload};
use super::super::houseblocks_v1::master::Master;
use failure::Error;
use std::cell::RefCell;
use std::time::Duration;

pub struct DeviceManager<'m> {
    master: &'m RefCell<Master>,
    address: Address,
}
impl<'m> DeviceManager<'m> {
    pub fn new(
        master: &'m RefCell<Master>,
        address: Address,
    ) -> Self {
        return Self { master, address };
    }
    pub async fn initialize(&self) -> Result<(), Error> {
        // Check whether device is not already initialized
        let status = self
            .master
            .borrow_mut()
            .transaction_out_in(
                false,
                self.address,
                Payload::new(Box::from(*b"")).unwrap(),
                Duration::from_secs(1),
            )
            .await?;

        log::debug!("status: {:#?}", status);

        return Ok(());
    }
}
