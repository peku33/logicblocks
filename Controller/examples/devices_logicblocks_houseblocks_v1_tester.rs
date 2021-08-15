#![feature(async_closure)]

use anyhow::{bail, Context, Error};
use futures::{
    future::{join, Future, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use logicblocks_controller::{
    devices::houseblocks::{
        avr_v1,
        houseblocks_v1::{
            common::{Address, AddressSerial},
            master::Master,
        },
    },
    interfaces::serial::ftdi::{Descriptor, Global},
    util::{async_flag::Sender, logging},
};
use std::time::Duration;
use tokio::signal::ctrl_c;

pub fn main() {
    logging::configure();

    main_error().unwrap();
}
fn execute_on_tokio<F: Future>(f: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
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
            0 => menu_masters_context().context("menu_masters_context")?,
            _ => panic!(),
        };
    }
    Ok(())
}
fn menu_masters_context() -> Result<(), Error> {
    let mut global = Global::new().context("global")?;
    let descriptors = global.find_descriptors().context("descriptors")?;

    let descriptor_names = descriptors
        .iter()
        .map(|descriptor| descriptor.serial_number.clone().into_string().unwrap())
        .collect::<Vec<_>>();

    let mut menu = dialoguer::Select::new();
    let menu = menu.with_prompt("Select Master").items(&descriptor_names);

    while let Some(result) = menu.interact_opt()? {
        menu_master_context(descriptors[result].clone()).context("menu_master_context")?;
    }
    Ok(())
}
fn menu_master_context(descriptor: Descriptor) -> Result<(), Error> {
    let master = Master::new(descriptor);

    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Select action")
        .item("Device discovery")
        .item("AVRv1 Context");

    while let Some(result) = menu.interact_opt()? {
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
    let mut menu = dialoguer::Select::new();
    let menu = menu.with_prompt("Select action").item("Run device");

    while let Some(result) = menu.interact_opt()? {
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
    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Device address")
        .default(0)
        .item("Detect automatically");

    while let Some(result) = menu.interact_opt()? {
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
            run_d0003_junction_box_minimal_v1(master, address.serial);
            Ok(())
        }
        b"0006" => {
            run_d0006_relay14_opto_a_v1(master, address.serial);
            Ok(())
        }
        b"0007" => {
            run_d0007_relay14_ssr_a_v2(master, address.serial);
            Ok(())
        }
        _ => bail!("device_type {} is not supported", address.device_type),
    }
}
fn run_d0003_junction_box_minimal_v1(
    master: &Master,
    address_serial: AddressSerial,
) {
    let runner = avr_v1::hardware::runner::Runner::<
        avr_v1::d0003_junction_box_minimal_v1::hardware::Device,
    >::new(master, address_serial);
    execute_on_tokio(async move {
        let runner_ref = &runner;
        async move {
            let avr_v1::d0003_junction_box_minimal_v1::hardware::PropertiesRemote {
                keys,
                leds,
                buzzer,
                temperature,
            } = runner_ref.properties_remote();

            let exit_flag_sender = Sender::new();

            let run_future = runner_ref.run(exit_flag_sender.receiver());

            let abort_runner = ctrl_c().then(async move |_| {
                exit_flag_sender.signal();
            });

            let keys_changed = || {
                let keys = match keys.take_pending() {
                    Some(keys) => keys,
                    None => return,
                };
                log::info!("keys: {:?}", keys);
            };

            let leds_runner = async {
                let mut led_index = 0;

                loop {
                    led_index += 1;
                    led_index %= avr_v1::d0003_junction_box_minimal_v1::hardware::LED_COUNT;

                    let mut led_values =
                        [false; avr_v1::d0003_junction_box_minimal_v1::hardware::LED_COUNT];
                    led_values[led_index] = true;

                    log::info!("setting leds: {:?}", led_values);
                    if leds.set(led_values) {
                        runner_ref.properties_remote_out_change_waker_wake();
                    }

                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            };
            pin_mut!(leds_runner);
            let mut leds_runner = leds_runner.fuse();

            let buzzer_runner = async {
                loop {
                    log::info!("pushing buzzer");
                    if buzzer.push(Duration::from_millis(125)) {
                        runner_ref.properties_remote_out_change_waker_wake();
                    }

                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            };
            pin_mut!(buzzer_runner);
            let mut buzzer_runner = buzzer_runner.fuse();

            let temperature_changed = || {
                let temperature = match temperature.take_pending() {
                    Some(temperature) => temperature,
                    None => return,
                };

                match temperature {
                    Some(temperature) => log::info!("temperature: {:?}", temperature),
                    None => log::warn!("temperature: None (Error)"),
                }
            };

            let properties_remote_in_changed_runner = async move {
                runner_ref
                    .properties_remote_in_change_waker_receiver()
                    .by_ref()
                    .for_each(|()| async move {
                        keys_changed();
                        temperature_changed();
                    }).await;
            };
            pin_mut!(properties_remote_in_changed_runner);
            let mut properties_remote_in_changed_runner = properties_remote_in_changed_runner.fuse();

            select! {
                _ = join(abort_runner, run_future).fuse() => {},
                _ = leds_runner => panic!("leds_runner"),
                _ = buzzer_runner => panic!("leds_runner"),
                _ = properties_remote_in_changed_runner => panic!("properties_remote_in_changed_runner yielded"),
            }
        }
        .await;
    });
}
fn run_d0006_relay14_opto_a_v1(
    master: &Master,
    address_serial: AddressSerial,
) {
    run_common_relay14_common::<avr_v1::d0006_relay14_opto_a_v1::hardware::Specification>(
        master,
        address_serial,
    )
}
fn run_d0007_relay14_ssr_a_v2(
    master: &Master,
    address_serial: AddressSerial,
) {
    run_common_relay14_common::<avr_v1::d0007_relay14_ssr_a_v2::hardware::Specification>(
        master,
        address_serial,
    )
}
fn run_common_relay14_common<S: avr_v1::common::relay14_common_a::hardware::Specification>(
    master: &Master,
    address_serial: AddressSerial,
) {
    let runner = avr_v1::hardware::runner::Runner::<
        avr_v1::common::relay14_common_a::hardware::Device<S>,
    >::new(master, address_serial);
    execute_on_tokio(async move {
        let runner_ref = &runner;
        async move {
            let avr_v1::common::relay14_common_a::hardware::PropertiesRemote { outputs } =
                runner_ref.properties_remote();

            let exit_flag_sender = Sender::new();

            let run_future = runner_ref.run(exit_flag_sender.receiver());

            let abort_runner = ctrl_c().then(async move |_| {
                exit_flag_sender.signal();
            });

            let outputs_runner = async {
                let mut output_index = 0;

                loop {
                    output_index += 1;
                    output_index %= avr_v1::common::relay14_common_a::hardware::OUTPUT_COUNT;

                    let mut output_values =
                        [false; avr_v1::common::relay14_common_a::hardware::OUTPUT_COUNT];
                    output_values[output_index] = true;

                    log::info!("setting outputs: {:?}", output_values);
                    if outputs.set(output_values) {
                        runner_ref.properties_remote_out_change_waker_wake();
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            };
            pin_mut!(outputs_runner);
            let mut outputs_runner = outputs_runner.fuse();

            select! {
                _ = join(abort_runner, run_future).fuse() => {},
                _ = outputs_runner => panic!("outputs_runner yielded"),
            }
        }
        .await;
    });
}
