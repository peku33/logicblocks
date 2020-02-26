use failure::Error;
use futures::future::{Future, FutureExt};
use futures::select;
use futures::stream::StreamExt;
use logicblocks_controller::devices::device::DeviceTrait;
use logicblocks_controller::devices::logicblocks::avr_v1;
use logicblocks_controller::devices::logicblocks::houseblocks_v1::common::AddressSerial;
use logicblocks_controller::devices::logicblocks::houseblocks_v1::master::{
    Master, MasterContext, MasterDescriptor,
};
use logicblocks_controller::util::ref_mut_async::FutureWrapper;
use std::cell::RefCell;
use std::convert::TryInto;
use std::time::Duration;

pub fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Trace)
        .init();

    main_error().unwrap();
}
fn execute_on_tokio<F: Future>(f: F) -> F::Output {
    let mut runtime = tokio::runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(f)
}
fn main_error() -> Result<(), Error> {
    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Select action")
        .item("Enter Master Context");

    while let Some(result) = menu.interact_opt()? {
        match result {
            0 => menu_masters_context(),
            _ => panic!(),
        }?;
    }
    Ok(())
}
fn menu_masters_context() -> Result<(), Error> {
    let master_context = MasterContext::new()?;
    let master_descriptors = master_context.find_master_descriptors()?;

    let master_descriptor_names = master_descriptors
        .iter()
        .map(|master_descriptor| {
            master_descriptor
                .serial_number
                .clone()
                .into_string()
                .unwrap()
        })
        .collect::<Vec<_>>();

    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Select Master")
        .items(&master_descriptor_names);

    while let Some(result) = menu.interact_opt()? {
        menu_master_context(&master_context, master_descriptors[result].clone())?;
    }
    Ok(())
}
fn menu_master_context(
    _master_context: &MasterContext,
    master_descriptor: MasterDescriptor,
) -> Result<(), Error> {
    let master = Master::new(master_descriptor)?;
    let master = RefCell::new(master);

    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Select action")
        .item("Device discovery")
        .item("AVRv1 Context");

    while let Some(result) = menu.interact_opt()? {
        match result {
            0 => menu_master_device_discovery(&master),
            1 => menu_master_avr_v1(&master),
            _ => panic!(),
        }?;
    }
    Ok(())
}
fn menu_master_device_discovery(master: &RefCell<Master>) -> Result<(), Error> {
    let master = master.borrow();
    execute_on_tokio(async move {
        let transaction = master.transaction_device_discovery();
        let address = transaction.await;
        match address {
            Ok(address) => log::info!("Address: {:?}", address),
            Err(error) => log::error!("Error: {}", error),
        };
    });
    Ok(())
}
fn menu_master_avr_v1(master: &RefCell<Master>) -> Result<(), Error> {
    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Select device type")
        .item("d0006_relay14_opto_a_v1")
        .item("d0007_relay14_ssr_a_v2");

    while let Some(result) = menu.interact_opt()? {
        match result {
            0 => menu_master_avr_v1_d0006_relay14_opto_a_v1(master),
            1 => menu_master_avr_v1_d0007_relay14_ssr_a_v2(master),
            _ => panic!(),
        }?;
    }

    Ok(())
}
fn ask_device_serial() -> Result<Option<AddressSerial>, Error> {
    let mut input = dialoguer::Input::<String>::new();
    let input = input.with_prompt("Serial");

    let address_serial = input.interact()?;
    if address_serial.is_empty() {
        return Ok(None);
    }
    let address_serial = AddressSerial::new(address_serial.as_bytes().try_into()?)?;
    Ok(Some(address_serial))
}
fn menu_master_avr_v1_d0006_relay14_opto_a_v1(master: &RefCell<Master>) -> Result<(), Error> {
    let address_serial = match ask_device_serial()? {
        Some(serial) => serial,
        None => return Ok(()),
    };
    let device = avr_v1::d0006_relay14_opto_a_v1::Device::new(master, address_serial);
    menu_master_avr_v1_relay14_common(&device)?;
    Ok(())
}
fn menu_master_avr_v1_d0007_relay14_ssr_a_v2(master: &RefCell<Master>) -> Result<(), Error> {
    let address_serial = match ask_device_serial()? {
        Some(serial) => serial,
        None => return Ok(()),
    };
    let device = avr_v1::d0007_relay14_ssr_a_v2::Device::new(master, address_serial);
    menu_master_avr_v1_relay14_common(&device)?;
    Ok(())
}
fn menu_master_avr_v1_relay14_common(device: &avr_v1::relay14_common::Device) -> Result<(), Error> {
    execute_on_tokio(async move {
        let device_run_object = device.device_run();
        let mut future_wrapper =
            FutureWrapper::new(device_run_object.get_run_future().borrow_mut()).fuse();

        let mut relay_id: usize = 0;
        let mut change_timer = tokio::time::interval(Duration::from_secs(1)).fuse();

        let mut ctrlc = tokio::signal::ctrl_c().boxed().fuse();

        loop {
            select! {
                _ = future_wrapper => panic!("future_wrapper exited"),
                _ = change_timer.next() => {
                    let mut relay_states = avr_v1::relay14_common::RelayStates::default();
                    relay_states.values[relay_id] = true;

                    log::info!("Setting relay states: {:?}", relay_states);
                    device.relay_states_set(relay_states);

                    relay_id += 1;
                    relay_id %= avr_v1::relay14_common::RELAYS;
                },
                _ = ctrlc => break,
            };
        }
    });
    Ok(())
}
