use super::super::houseblocks_v1::common::{AddressDeviceType, AddressSerial};
use super::super::houseblocks_v1::master::Master;
use super::relay14_common::Device;
use std::cell::RefCell;

pub fn new<'m>(
    master: &'m RefCell<Master>,
    address_serial: AddressSerial,
) -> Device<'m> {
    return Device::new(
        master,
        AddressDeviceType::new_from_ordinal(6).unwrap(),
        address_serial,
        "logicblocks/avr_v1/0006_relay14_opto_a",
    );
}
