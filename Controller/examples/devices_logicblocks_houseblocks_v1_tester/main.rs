#![feature(async_closure)]

pub mod common;
pub mod d0002_reed_switch_v1;
pub mod d0003_junction_box_minimal_v1;
pub mod d0005_gpio_a_v1;
pub mod d0006_relay14_opto_a_v1;
pub mod d0007_relay14_ssr_a_v2;

use anyhow::{Context, Error};
use common::execute_on_tokio;
use logicblocks_controller::{
    devices::houseblocks::houseblocks_v1::{
        common::{Address, AddressDeviceType, AddressSerial},
        master::Master,
    },
    interfaces::serial::ftdi::Global,
    util::logging,
};

pub fn main() {
    logging::configure(module_path!());

    main_error().unwrap();
}

fn main_error() -> Result<(), Error> {
    let mut global = Global::new().context("global")?;

    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Select action")
        .default(0)
        .item("Select Master")
        .interact_opt()?
    {
        match result {
            0 => {
                let master =
                    match submenu_select_master(&mut global).context("submenu_select_master")? {
                        Some(master) => master,
                        None => continue,
                    };

                menu_master_context(&master)?;
            }
            _ => panic!(),
        };
    }

    Ok(())
}
fn menu_master_context(master: &Master) -> Result<(), Error> {
    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Select action")
        .default(0)
        .item("Device discovery")
        .item("Run Device")
        .interact_opt()?
    {
        match result {
            0 => match master_device_discovery(master) {
                Ok(address) => log::info!("address: {}", address),
                Err(error) => log::error!("error: {:?}", error),
            },
            1 => {
                let address = match submenu_ask_device_address(master)
                    .context("submenu_ask_device_address")?
                {
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
fn run_by_address(
    master: &Master,
    address: Address,
) -> Result<(), Error> {
    match address.device_type.as_bytes() {
        b"0002" => {
            d0002_reed_switch_v1::run(master, address.serial)?;
        }
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
        _ => {
            log::warn!("device_type {} is not supported", address.device_type);
        }
    }

    Ok(())
}

fn submenu_select_master(global: &mut Global) -> Result<Option<Master>, Error> {
    let descriptors = global.find_descriptors().context("descriptors")?;
    let descriptors = descriptors.all();

    if descriptors.is_empty() {
        log::warn!("no descriptors found");
        return Ok(None);
    }

    let index = match dialoguer::Select::new()
        .with_prompt("Select Master")
        .default(0)
        .items(
            descriptors
                .iter()
                .map(|descriptor| descriptor.serial_number.clone().into_string().unwrap())
                .collect::<Vec<_>>()
                .as_ref(),
        )
        .interact_opt()?
    {
        Some(index) => index,
        None => return Ok(None),
    };

    let descriptor = &descriptors[index];
    let master = Master::new(descriptor.clone());

    Ok(Some(master))
}
fn submenu_ask_device_address(master: &Master) -> Result<Option<Address>, Error> {
    while let Some(result) = dialoguer::Select::new()
        .with_prompt("Device address")
        .default(0)
        .item("Detect automatically")
        .item("Provide manually")
        .interact_opt()?
    {
        match result {
            0 => match master_device_discovery(master).context("master_device_discovery") {
                Ok(address) => {
                    log::info!("detected address: {}", address);

                    return Ok(Some(address));
                }
                Err(error) => {
                    log::error!("error while detecting serial: {:?}", error);
                    continue;
                }
            },
            1 => match submenu_ask_device_address_manual()
                .context("submenu_ask_device_address_manual")
            {
                Ok(Some(address)) => return Ok(Some(address)),
                Ok(None) => continue,
                Err(error) => {
                    log::error!("error while detecting serial: {:?}", error);
                    continue;
                }
            },
            _ => panic!(),
        }
    }

    Ok(None)
}
fn submenu_ask_device_address_manual() -> Result<Option<Address>, Error> {
    let device_type_string = dialoguer::Input::<String>::new()
        .with_prompt("Device Type [4 digits]")
        .allow_empty(true)
        .interact_text()?;
    if device_type_string.is_empty() {
        return Ok(None);
    }
    let device_type =
        AddressDeviceType::new_from_string(&device_type_string).context("new_from_ordinal")?;

    let serial_string = dialoguer::Input::<String>::new()
        .with_prompt("Device Serial [8 digits]")
        .allow_empty(true)
        .interact_text()?;
    if serial_string.is_empty() {
        return Ok(None);
    }
    let serial = AddressSerial::new_from_string(&serial_string).context("new_from_ordinal")?;

    let address = Address {
        device_type,
        serial,
    };
    Ok(Some(address))
}

fn master_device_discovery(master: &Master) -> Result<Address, Error> {
    execute_on_tokio(async move {
        let transaction = master.transaction_device_discovery();
        transaction.await
    })
}
