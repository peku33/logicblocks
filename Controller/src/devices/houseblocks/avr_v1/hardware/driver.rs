use super::{
    super::super::houseblocks_v1::{
        common::{Address, Payload},
        master::Master,
    },
    parser::Parser,
    serializer::Serializer,
};
use anyhow::{Context, Error, ensure};
use crc::{CRC_16_MODBUS, Crc};
use std::time::Duration;

#[derive(Debug)]
pub struct PowerFlags {
    wdt: bool,
    bod: bool,
    ext_reset: bool,
    pon: bool,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Version {
    avr_v1: u16,
    application: u16,
}

#[derive(Debug)]
pub struct Firmware<'c> {
    content: &'c [u8],
    checksum: u16,
}
impl<'c> Firmware<'c> {
    pub const SIZE_BYTES: usize = 6144;

    const CHECKSUM_HASHER: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
    fn checksum(content: &[u8]) -> u16 {
        let mut checksum = Self::CHECKSUM_HASHER.digest();

        // actual payload
        checksum.update(content);

        // padding
        for _ in content.len()..Self::SIZE_BYTES {
            checksum.update(&[0u8]);
        }

        let checksum = checksum.finalize();

        checksum
    }

    pub fn new(content: &'c [u8]) -> Self {
        assert!(content.len() <= Self::SIZE_BYTES);

        let checksum = Self::checksum(content);

        Self { content, checksum }
    }
}

#[derive(Debug)]
pub struct PrepareResult {
    application_checksum: u16,
    version: Version,
}

#[derive(Debug)]
pub struct Driver<'m> {
    master: &'m Master,
    address: Address,
}
impl<'m> Driver<'m> {
    const TIMEOUT_DEFAULT: Duration = Duration::from_millis(250);
    const AVR_V1_VERSION_SUPPORTED: u16 = 4;
    const SERVICE_MODE_VERSION_SUPPORTED: Version = Version {
        avr_v1: Self::AVR_V1_VERSION_SUPPORTED,
        application: 2,
    };

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
            .await
            .context("transaction_out")?;

        Ok(())
    }

    async fn transaction_out_in(
        &self,
        service_mode: bool,
        payload: Payload,
        timeout_custom: Option<Duration>,
    ) -> Result<Payload, Error> {
        let result = self
            .master
            .transaction_out_in(
                service_mode,
                self.address,
                payload,
                timeout_custom.unwrap_or(Self::TIMEOUT_DEFAULT),
            )
            .await
            .context("transaction_out_in")?;

        Ok(result)
    }

    // Routines
    async fn healthcheck(
        &self,
        service_mode: bool,
    ) -> Result<(), Error> {
        let response = self
            .transaction_out_in(service_mode, Payload::new(Box::from(*b"")).unwrap(), None)
            .await
            .context("transaction_out_in")?;

        ensure!(
            response.as_bytes() == &b""[..],
            "invalid healthcheck response"
        );

        Ok(())
    }

    async fn reboot(
        &self,
        service_mode: bool,
    ) -> Result<(), Error> {
        self.transaction_out(service_mode, Payload::new(Box::from(*b"!")).unwrap())
            .await
            .context("transaction_out")?;

        tokio::time::sleep(Self::TIMEOUT_DEFAULT).await;
        Ok(())
    }

    async fn read_clear_power_flags(
        &self,
        service_mode: bool,
    ) -> Result<PowerFlags, Error> {
        let response = self
            .transaction_out_in(service_mode, Payload::new(Box::from(*b"@")).unwrap(), None)
            .await
            .context("transaction_out_in")?;

        let mut parser = Parser::from_payload(&response);
        let wdt = parser.expect_bool().context("wdt")?;
        let bod = parser.expect_bool().context("bod")?;
        let ext_reset = parser.expect_bool().context("ext_reset")?;
        let pon = parser.expect_bool().context("pon")?;
        parser.expect_end().context("expect_end")?;

        Ok(PowerFlags {
            wdt,
            bod,
            ext_reset,
            pon,
        })
    }

    async fn read_version(
        &self,
        service_mode: bool,
    ) -> Result<Version, Error> {
        let response = self
            .transaction_out_in(service_mode, Payload::new(Box::from(*b"#")).unwrap(), None)
            .await
            .context("transaction_out_in")?;

        let mut parser = Parser::from_payload(&response);
        let avr_v1 = parser.expect_u16().context("avr_v1")?;
        let application = parser.expect_u16().context("application")?;
        parser.expect_end().context("expect_end")?;

        Ok(Version {
            avr_v1,
            application,
        })
    }

    // Service mode routines
    async fn service_mode_read_application_checksum(&self) -> Result<u16, Error> {
        let response = self
            .transaction_out_in(true, Payload::new(Box::new(*b"C")).unwrap(), None)
            .await
            .context("transaction_out_in")?;

        let mut parser = Parser::from_payload(&response);
        let checksum = parser.expect_u16().context("checksum")?;
        parser.expect_end().context("expect_end")?;

        Ok(checksum)
    }
    async fn service_mode_write_application(
        &self,
        content: &[u8],
    ) -> Result<(), Error> {
        assert!(content.len() <= Firmware::SIZE_BYTES);

        const PAGE_SIZE_BYTES: usize = 64;

        for page_id in 0..(Firmware::SIZE_BYTES / PAGE_SIZE_BYTES) {
            let mut page = [0u8; PAGE_SIZE_BYTES];

            // generate subrange of original content, rest will be filled by 0 (in page
            // constructor)
            let content_start = (page_id * PAGE_SIZE_BYTES).min(content.len());
            let content_end = ((page_id + 1) * PAGE_SIZE_BYTES).min(content.len());
            if content_start != content_end {
                page[0..(content_end - content_start)]
                    .copy_from_slice(&content[content_start..content_end]);
            }

            let mut serializer = Serializer::new();
            serializer.push_byte(b'W');
            serializer.push_u8(page_id as u8);
            for page_byte in page {
                serializer.push_u8(page_byte);
            }
            let payload = serializer.into_payload();

            self.transaction_out_in(true, payload, None)
                .await
                .context("transaction_out_in")?;
        }

        Ok(())
    }
    async fn service_mode_jump_to_application_mode(&self) -> Result<(), Error> {
        self.transaction_out(true, Payload::new(Box::from(*b"R")).unwrap())
            .await
            .context("transaction_out")?;

        tokio::time::sleep(Duration::from_millis(250)).await;
        Ok(())
    }

    // Procedures
    pub async fn prepare(
        &self,
        firmware: Option<&Firmware<'_>>,
        application_version_supported: Option<u16>,
    ) -> Result<PrepareResult, Error> {
        // Driver may be already initialized, check it.
        let healthcheck_result = self.healthcheck(false).await;
        if healthcheck_result.is_ok() {
            // Is initialized, perform reboot
            self.reboot(false).await.context("deinitialize reboot")?;
        }

        // Check if we can talk to the bootloader
        let service_mode_version = self
            .read_version(true)
            .await
            .context("service mode read version")?;
        ensure!(
            service_mode_version == Self::SERVICE_MODE_VERSION_SUPPORTED,
            "service mode version not supported, got {:?}, expecting {:?}",
            service_mode_version,
            Self::SERVICE_MODE_VERSION_SUPPORTED
        );

        // Perform application update if needed
        let mut application_checksum = self
            .service_mode_read_application_checksum()
            .await
            .context("service mode read application checksum")?;
        if let Some(firmware) = firmware.as_ref()
            && application_checksum != firmware.checksum
        {
            log::info!(
                "performing firmware upgrade on {} ({:04X} -> {:04X})",
                self.address,
                application_checksum,
                firmware.checksum
            );

            // Write the firmware
            self.service_mode_write_application(firmware.content)
                .await
                .context("service mode write application")?;

            // Verify it
            application_checksum = self
                .service_mode_read_application_checksum()
                .await
                .context("service mode read application checksum")?;

            ensure!(
                application_checksum == firmware.checksum,
                "application checksum mismatch after update"
            );

            log::info!(
                "firmware upgrade completed successfully on {}",
                self.address,
            );
        }

        // Reboot to application section
        self.service_mode_jump_to_application_mode()
            .await
            .context("jump to application mode")?;

        // Verify application version (this also verifies liveness)
        let version = self.read_version(false).await.context("read version")?;

        if let Some(application_version_supported) = application_version_supported {
            let version_expected = Version {
                avr_v1: Self::AVR_V1_VERSION_SUPPORTED,
                application: application_version_supported,
            };

            ensure!(
                version == version_expected,
                "version not supported, got {:?}, expecting {:?}",
                version,
                version_expected
            );
        }

        let prepare_result = PrepareResult {
            application_checksum,
            version,
        };

        Ok(prepare_result)
    }
}

#[derive(Debug)]
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
        timeout_custom: Option<Duration>,
    ) -> Result<Payload, Error> {
        self.driver
            .transaction_out_in(false, payload, timeout_custom)
            .await
    }
}
