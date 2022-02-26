#![feature(async_closure)]

pub mod common;
pub mod d0003_junction_box_minimal_v1;
pub mod d0005_gpio_a_v1;
pub mod d0006_relay14_opto_a_v1;
pub mod d0007_relay14_ssr_a_v2;

use anyhow::{bail, Context, Error};
use common::execute_on_tokio;
use logicblocks_controller::{
    devices::houseblocks::houseblocks_v1::{common::Address, master::Master},
    interfaces::serial::ftdi::{Descriptor, Global},
    util::logging,
};

pub fn main() {
    logging::configure(module_path!());

    main_error().unwrap();
}

fn main_error() -> Result<(), Error> {
    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Select action")
        .item("Enter Master Context")
        .interact_opt()?
    {
        match result {
            0 => menu_masters_context().context("menu_masters_context")?,
            _ => panic!(),
        };
    }
    Ok(())
}
fn menu_masters_context() -> Result<(), Error> {
    let mut global = Global::new().context("global")?;
    let descriptors = global.find_descriptors().context("descriptors")?;

    if descriptors.is_empty() {
        eprintln!("no descriptors found");
        return Ok(());
    }

    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Select Master")
        .items(
            descriptors
                .iter()
                .map(|descriptor| descriptor.serial_number.clone().into_string().unwrap())
                .collect::<Vec<_>>()
                .as_ref(),
        )
        .interact_opt()?
    {
        menu_master_context(descriptors[result].clone()).context("menu_master_context")?;
    }
    Ok(())
}
fn menu_master_context(descriptor: Descriptor) -> Result<(), Error> {
    let master = Master::new(descriptor);

    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Select action")
        .item("Device discovery")
        .item("AVRv1 Context")
        .interact_opt()?
    {
        match result {
            0 => menu_master_device_discovery(&master),
            1 => menu_master_avr_v1(&master).context("menu_master_avr_v1")?,
            _ => panic!(),
        };
    }
    Ok(())
}
fn master_device_discovery(master: &Master) -> Result<Address, Error> {
    execute_on_tokio(async move {
        let transaction = master.transaction_device_discovery();
        transaction.await
    })
}
fn menu_master_device_discovery(master: &Master) {
    match master_device_discovery(master).context("master_device_discovery") {
        Ok(address) => println!("address: {}", address),
        Err(error) => log::error!("error: {:?}", error),
    };
}
fn menu_master_avr_v1(master: &Master) -> Result<(), Error> {
    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Select action")
        .item("Run device")
        .interact_opt()?
    {
        match result {
            0 => {
                let address = match ask_device_serial(master).context("ask_device_serial")? {
                    Some(address) => address,
                    None => continue,
                };
                run_by_address(master, address).context("run_by_address")?;
            }
            _ => panic!(),
        };
    }

    Ok(())
}
fn ask_device_serial(master: &Master) -> Result<Option<Address>, Error> {
    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Device address")
        .default(0)
        .item("Detect automatically")
        .interact_opt()?
    {
        match result {
            0 => match master_device_discovery(master).context("master_device_discovery") {
                Ok(address_serial) => {
                    log::info!("detected address: {}", address_serial);
                    return Ok(Some(address_serial));
                }
                Err(error) => {
                    log::error!("error while detecting serial: {:?}", error);
                }
            },
            _ => panic!(),
        }
    }
    Ok(None)
}
fn run_by_address(
    master: &Master,
    address: Address,
) -> Result<(), Error> {
    match address.device_type.as_bytes() {
        b"0003" => {
            d0003_junction_box_minimal_v1::run(master, address.serial)?;
        }
        b"0005" => {
            d0005_gpio_a_v1::menu(master, address.serial)?;
        }
        b"0006" => {
            d0006_relay14_opto_a_v1::run(master, address.serial)?;
        }
        b"0007" => {
            d0007_relay14_ssr_a_v2::run(master, address.serial)?;
        }
        _ => bail!("device_type {} is not supported", address.device_type),
    }

    Ok(())
}
