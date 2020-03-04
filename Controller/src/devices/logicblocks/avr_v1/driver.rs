use super::super::houseblocks_v1::common::{Address, Payload};
use super::super::houseblocks_v1::master::Master;
use failure::{err_msg, format_err, Error};
use std::cell::RefCell;
use std::convert::TryInto;
use std::ops::Deref;
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
pub struct Driver<'m> {
    master: &'m RefCell<Master>,
    address: Address,
}
impl<'m> Driver<'m> {
    pub fn new(
        master: &'m RefCell<Master>,
        address: Address,
    ) -> Self {
        Self { master, address }
    }

    // Transactions
    async fn transaction_out(
        &self,
        service_mode: bool,
        payload: Payload,
    ) -> Result<(), Error> {
        self.master
            .borrow()
            .transaction_out(service_mode, self.address, payload)
            .await?;

        Ok(())
    }

    async fn transaction_out_in(
        &self,
        service_mode: bool,
        payload: Payload,
        timeout: Duration,
    ) -> Result<Payload, Error> {
        let result = self
            .master
            .borrow()
            .transaction_out_in(service_mode, self.address, payload, timeout)
            .await?;

        Ok(result)
    }

    // Routines
    async fn healthcheck(
        &self,
        service_mode: bool,
    ) -> Result<(), Error> {
        let response = self
            .transaction_out_in(
                service_mode,
                Payload::new(Box::from(*b"")).unwrap(),
                Duration::from_millis(250),
            )
            .await?;

        // TODO: Is .deref() call legal?
        if response.deref() != &b""[..] {
            return Err(err_msg("invalid healthcheck response"));
        }
        Ok(())
    }

    async fn reboot(
        &self,
        service_mode: bool,
    ) -> Result<(), Error> {
        self.transaction_out(service_mode, Payload::new(Box::from(*b"!")).unwrap())
            .await?;

        tokio::time::delay_for(Duration::from_millis(250)).await;
        Ok(())
    }

    async fn read_clear_power_flags(
        &self,
        service_mode: bool,
    ) -> Result<PowerFlags, Error> {
        let response = self
            .transaction_out_in(
                service_mode,
                Payload::new(Box::from(*b"@")).unwrap(),
                Duration::from_millis(250),
            )
            .await?;

        if response.len() != 4 {
            return Err(format_err!(
                "invalid power_flags response length ({})",
                response.len()
            ));
        }

        Ok(PowerFlags {
            wdt: flag10_to_bool(response[0])?,
            bod: flag10_to_bool(response[1])?,
            ext_reset: flag10_to_bool(response[2])?,
            pon: flag10_to_bool(response[3])?,
        })
    }

    async fn read_application_version(
        &self,
        service_mode: bool,
    ) -> Result<Version, Error> {
        let response = self
            .transaction_out_in(
                service_mode,
                Payload::new(Box::from(*b"#")).unwrap(),
                Duration::from_millis(250),
            )
            .await?;

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

        Ok(Version {
            avr_v1,
            application,
        })
    }

    // Service mode routines
    async fn service_mode_read_application_checksum(&self) -> Result<u16, Error> {
        let response = self
            .transaction_out_in(
                true,
                Payload::new(Box::new(*b"C")).unwrap(),
                Duration::from_millis(250),
            )
            .await?;

        if response.len() != 4 {
            return Err(format_err!(
                "invalid checksum response length ({})",
                response.len()
            ));
        }

        let checksum = hex::decode(&response.as_ref())?;
        let checksum = u16::from_be_bytes((&checksum[..]).try_into().unwrap());

        Ok(checksum)
    }
    async fn service_mode_jump_to_application_mode(&self) -> Result<(), Error> {
        self.transaction_out(true, Payload::new(Box::from(*b"R")).unwrap())
            .await?;

        tokio::time::delay_for(Duration::from_millis(250)).await;
        Ok(())
    }

    // Procedures
    pub async fn initialize(&self) -> Result<ApplicationModeDriver<'m, '_>, Error> {
        // Driver may be already initialized, check it.
        let healthcheck_result = self.healthcheck(false).await;
        if healthcheck_result.is_ok() {
            log::info!("{:?}: driver was already initialized, rebooting", self);
            self.reboot(false).await?;
        }

        // We should be in service mode
        self.healthcheck(true).await?;

        // Check application up to date
        let application_checksum = self.service_mode_read_application_checksum().await?;
        log::trace!("{:?}: application_checksum: {}", self, application_checksum);
        // TODO: Push new firmware

        // Reboot to application section
        self.service_mode_jump_to_application_mode().await?;

        // Check life in application section
        self.healthcheck(false).await?;

        Ok(ApplicationModeDriver::new(self))
    }
}

fn flag10_to_bool(value: u8) -> Result<bool, Error> {
    match value {
        b'0' => Ok(false),
        b'1' => Ok(true),
        _ => Err(err_msg("invalid bool flag value")),
    }
}

pub struct ApplicationModeDriver<'m, 'd> {
    driver: &'d Driver<'m>,
}
impl<'m, 'd> ApplicationModeDriver<'m, 'd> {
    fn new(driver: &'d Driver<'m>) -> Self {
        Self { driver }
    }

    // Transactions
    pub async fn transaction_out(
        &self,
        payload: Payload,
    ) -> Result<(), Error> {
        self.driver.transaction_out(false, payload).await
    }

    pub async fn transaction_out_in(
        &self,
        payload: Payload,
        timeout: Duration,
    ) -> Result<Payload, Error> {
        self.driver
            .transaction_out_in(false, payload, timeout)
            .await
    }

    // Routines
    pub async fn healthcheck(&self) -> Result<(), Error> {
        self.driver.healthcheck(false).await
    }

    pub async fn reboot(&self) -> Result<(), Error> {
        self.driver.reboot(false).await
    }

    pub async fn read_clear_power_flags(&self) -> Result<PowerFlags, Error> {
        self.driver.read_clear_power_flags(false).await
    }

    pub async fn read_application_version(&self) -> Result<Version, Error> {
        self.driver.read_application_version(false).await
    }
}
