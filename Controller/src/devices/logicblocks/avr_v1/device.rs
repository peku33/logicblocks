#![cfg(target_os = "linux")]

use super::super::houseblocks_v1::common::{Address, Payload};
use super::super::houseblocks_v1::master::Master;
use failure::{err_msg, format_err, Error};
use std::cell::RefCell;
use std::convert::TryInto;
use std::time::Duration;

#[derive(Debug)]
pub struct PowerFlags {
    wdt: bool,
    bod: bool,
    ext_reset: bool,
    pon: bool,
}

#[derive(Debug)]
pub struct Version {
    avr_v1: u16,
    application: u16,
}

#[derive(Debug)]
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

    pub async fn healthcheck(
        &self,
        service_mode: bool,
    ) -> Result<(), Error> {
        let response = self
            .master
            .borrow_mut()
            .transaction_out_in(
                service_mode,
                self.address,
                Payload::new(Box::from(*b"")).unwrap(),
                Duration::from_millis(250),
            )
            .await?;

        if response.as_slice() != &b""[..] {
            return Err(err_msg("invalid healthcheck response"));
        }
        return Ok(());
    }

    pub async fn reboot(
        &self,
        service_mode: bool,
    ) -> Result<(), Error> {
        self.master
            .borrow_mut()
            .transaction_out(
                service_mode,
                self.address,
                Payload::new(Box::from(*b"!")).unwrap(),
            )
            .await?;

        return Ok(());
    }

    pub async fn read_clear_power_flags(
        &self,
        service_mode: bool,
    ) -> Result<PowerFlags, Error> {
        let response = self
            .master
            .borrow_mut()
            .transaction_out_in(
                service_mode,
                self.address,
                Payload::new(Box::from(*b"@")).unwrap(),
                Duration::from_millis(250),
            )
            .await?;

        let response = response.as_slice();
        if response.len() != 4 {
            return Err(format_err!(
                "invalid power_flags response length ({})",
                response.len()
            ));
        }

        return Ok(PowerFlags {
            wdt: flag10_to_bool(response[0])?,
            bod: flag10_to_bool(response[1])?,
            ext_reset: flag10_to_bool(response[2])?,
            pon: flag10_to_bool(response[3])?,
        });
    }

    pub async fn read_application_version(
        &self,
        service_mode: bool,
    ) -> Result<Version, Error> {
        let response = self
            .master
            .borrow_mut()
            .transaction_out_in(
                service_mode,
                self.address,
                Payload::new(Box::from(*b"#")).unwrap(),
                Duration::from_millis(250),
            )
            .await?;

        let response = response.as_slice();
        if response.len() != 8 {
            return Err(format_err!(
                "invalid version response length ({})",
                response.len()
            ));
        }

        let avr_v1 = hex::decode(&response[0..4])?;
        let avr_v1 = u16::from_be_bytes((&avr_v1[..]).try_into().unwrap());

        let application = hex::decode(&response[4..8])?;
        let application = u16::from_be_bytes((&application[..]).try_into().unwrap());

        return Ok(Version {
            avr_v1,
            application,
        });
    }
}

fn flag10_to_bool(value: u8) -> Result<bool, Error> {
    return match value {
        b'0' => Ok(false),
        b'1' => Ok(true),
        _ => Err(err_msg("invalid bool flag value")),
    };
}
