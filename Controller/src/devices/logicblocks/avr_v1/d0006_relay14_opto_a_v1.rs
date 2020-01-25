use super::super::houseblocks_v1::common::{AddressDeviceType, AddressSerial};
use super::super::houseblocks_v1::master::Master;
use super::relay14_common::Device as CommonDevice;
use crate::devices::device::{AsDeviceTrait, DeviceTrait};
use std::cell::RefCell;

pub struct Device<'m> {
    common_device: CommonDevice<'m>,
}
impl<'m> Device<'m> {
    pub fn new(
        master: &'m RefCell<Master>,
        address_serial: AddressSerial,
    ) -> Self {
        Self {
            common_device: CommonDevice::new(
                master,
                AddressDeviceType::new_from_ordinal(6).unwrap(),
                address_serial,
                "logicblocks/avr_v1/0006_relay14_opto_a_v1",
            ),
        }
    }
}
impl<'m> AsDeviceTrait for Device<'m> {
    fn as_device_trait(&self) -> &dyn DeviceTrait {
        &self.common_device
    }
}
