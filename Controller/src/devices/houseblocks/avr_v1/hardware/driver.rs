use super::{
    super::super::houseblocks_v1::{
        common::{Address, Payload},
        master::Master,
    },
    parser::{Parser, ParserPayload},
};
use anyhow::{bail, Context, Error};
use std::{ops::Deref, time::Duration};

const TIMEOUT_DEFAULT: Duration = Duration::from_millis(250);

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
    master: &'m Master,
    address: Address,
}
impl<'m> Driver<'m> {
    pub fn new(
        master: &'m Master,
        address: Address,
    ) -> Self {
        Self { master, address }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    // Transactions
    async fn transaction_out(
        &self,
        service_mode: bool,
        payload: Payload,
    ) -> Result<(), Error> {
        self.master
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
                TIMEOUT_DEFAULT,
            )
            .await?;

        if response.deref() != &b""[..] {
            bail!("invalid healthcheck response");
        }
        Ok(())
    }

    async fn reboot(
        &self,
        service_mode: bool,
    ) -> Result<(), Error> {
        self.transaction_out(service_mode, Payload::new(Box::from(*b"!")).unwrap())
            .await?;

        tokio::time::delay_for(TIMEOUT_DEFAULT).await;
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
                TIMEOUT_DEFAULT,
            )
            .await?;

        let mut parser = ParserPayload::new(&response);
        let wdt = parser.expect_bool().context("wdt")?;
        let bod = parser.expect_bool().context("bod")?;
        let ext_reset = parser.expect_bool().context("ext_reset")?;
        let pon = parser.expect_bool().context("pon")?;
        parser.expect_end()?;

        Ok(PowerFlags {
            wdt,
            bod,
            ext_reset,
            pon,
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
                TIMEOUT_DEFAULT,
            )
            .await?;

        let mut parser = ParserPayload::new(&response);
        let avr_v1 = parser.expect_u16().context("avr_v1")?;
        let application = parser.expect_u16().context("application")?;
        parser.expect_end()?;

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
                TIMEOUT_DEFAULT,
            )
            .await?;

        let mut parser = ParserPayload::new(&response);
        let checksum = parser.expect_u16().context("checksum")?;
        parser.expect_end()?;

        Ok(checksum)
    }
    async fn service_mode_jump_to_application_mode(&self) -> Result<(), Error> {
        self.transaction_out(true, Payload::new(Box::from(*b"R")).unwrap())
            .await?;

        tokio::time::delay_for(Duration::from_millis(250)).await;
        Ok(())
    }

    // Procedures
    pub async fn prepare(&self) -> Result<(), Error> {
        // Driver may be already initialized, check it.
        let healthcheck_result = self.healthcheck(false).await;
        if healthcheck_result.is_ok() {
            // Is initialized, perform reboot
            self.reboot(false).await.context("deinitialize reboot")?;
        }

        // We should be in service mode
        self.healthcheck(true)
            .await
            .context("service mode healthcheck")?;

        // Check application up to date
        let application_checksum = self
            .service_mode_read_application_checksum()
            .await
            .context("service mode read application checksum")?;
        log::trace!("application_checksum: {}", application_checksum);
        // TODO: Push new firmware

        // Reboot to application section
        self.service_mode_jump_to_application_mode()
            .await
            .context("jump to application mode")?;

        // Check life in application section
        self.healthcheck(false)
            .await
            .context("application mode healthcheck")?;

        Ok(())
    }
}

pub struct ApplicationDriver<'d> {
    driver: &'d Driver<'d>,
}
impl<'d> ApplicationDriver<'d> {
    pub fn new(driver: &'d Driver<'d>) -> Self {
        Self { driver }
    }

    pub async fn transaction_out(
        &self,
        payload: Payload,
    ) -> Result<(), Error> {
        self.driver.transaction_out(false, payload).await
    }

    pub async fn transaction_out_in(
        &self,
        payload: Payload,
        timeout: Option<Duration>,
    ) -> Result<Payload, Error> {
        self.driver
            .transaction_out_in(false, payload, timeout.unwrap_or(TIMEOUT_DEFAULT))
            .await
    }
}
