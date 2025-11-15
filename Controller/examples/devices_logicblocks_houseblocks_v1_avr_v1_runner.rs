use anyhow::{Context, Error};
use clap::Parser;
use logicblocks_controller::{
    devices::houseblocks::{
        avr_v1::hardware::driver::{Driver, Firmware},
        houseblocks_v1::{
            common::{Address, AddressDeviceType, AddressSerial},
            master::Master,
        },
    },
    interfaces::serial::ftdi,
    util::logging,
};
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Arguments {
    ftdi_serial: String,
    address_device_type: AddressDeviceType,
    address_serial: AddressSerial,

    #[clap(subcommand)]
    subcommand: ArgumentsSubcommand,
}

#[derive(Debug, Parser)]
enum ArgumentsSubcommand {
    Prepare(ArgumentsSubcommandPrepare),
}

#[derive(Debug, Parser)]
#[clap(name = "prepare")]
struct ArgumentsSubcommandPrepare {
    #[arg(short, long)]
    firmware: Option<PathBuf>,

    #[arg(short, long)]
    application_version_supported: Option<u16>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!(), true);

    let arguments = Arguments::parse();

    let mut ftdi_global_context = ftdi::Global::new().context("ftdi_global_context")?;
    let ftdi_descriptors = ftdi_global_context
        .find_descriptors()
        .context("find_descriptors")?;

    let ftdi_descriptor = ftdi_descriptors.descriptor_by_serial_or_error(&arguments.ftdi_serial)?;

    let master = Master::new(ftdi_descriptor.clone());

    let address = Address {
        device_type: arguments.address_device_type,
        serial: arguments.address_serial,
    };

    let driver = Driver::new(&master, address);

    match arguments.subcommand {
        ArgumentsSubcommand::Prepare(subcommand_arguments) => {
            let firmware = if let Some(firmware) = subcommand_arguments.firmware {
                let firmware = tokio::fs::read(firmware).await.context("firmware read")?;

                Some(firmware)
            } else {
                None
            };
            let firmware = firmware.as_ref().map(|firmware| Firmware::new(firmware));

            let application_version_supported = subcommand_arguments.application_version_supported;

            let prepare_result = driver
                .prepare(firmware.as_ref(), application_version_supported)
                .await
                .context("prepare")?;

            log::info!("prepare_result: {prepare_result:?}");
        }
    }

    Ok(())
}
