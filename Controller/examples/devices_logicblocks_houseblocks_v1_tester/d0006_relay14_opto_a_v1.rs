use super::common::relay14_common_a::run as run_common_relay14_common_a;
use anyhow::Error;
use logicblocks_controller::devices::houseblocks::{
    avr_v1::devices::d0006_relay14_opto_a_v1::hardware::Specification,
    houseblocks_v1::{common::AddressSerial, master::Master},
};

pub fn run(
    master: &Master,
    address_serial: AddressSerial,
) -> Result<(), Error> {
    run_common_relay14_common_a::<Specification>(master, address_serial)?;
    Ok(())
}
