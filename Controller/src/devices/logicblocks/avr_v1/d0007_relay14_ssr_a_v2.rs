use super::super::houseblocks_v1::common::{AddressDeviceType, AddressSerial};
use super::super::houseblocks_v1::master::Master;
use super::relay14_common::Device as CommonDevice;
use crate::devices::device::{AsDeviceTrait, DeviceTrait};
use std::cell::RefCell;
use std::ops::Deref;

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
                AddressDeviceType::new_from_ordinal(7).unwrap(),
                address_serial,
                "logicblocks/avr_v1/0007_relay14_ssr_a_v2",
            ),
        }
    }
}
impl<'m> Deref for Device<'m> {
    type Target = CommonDevice<'m>;
    fn deref(&self) -> &Self::Target {
        &self.common_device
    }
}
impl<'m> AsDeviceTrait for Device<'m> {
    fn as_device_trait(&self) -> &dyn DeviceTrait {
        &self.common_device
    }
}
