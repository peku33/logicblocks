use anyhow::{bail, Context, Error};
use futures::{
    future::{Future, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use logicblocks_controller::{
    devices::houseblocks::{
        avr_v1,
        houseblocks_v1::{
            common::{Address, AddressDeviceType, AddressSerial},
            master::Master,
        },
    },
    interfaces::serial::ftdi::{Descriptor, Global},
};
use std::{convert::TryInto, time::Duration};

pub fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Debug)
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
            0 => menu_master_device_discovery(&master).context("menu_master_device_discovery")?,
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
fn menu_master_device_discovery(master: &Master) -> Result<(), Error> {
    match master_device_discovery(master) {
        Ok(address) => println!("address: {}", address),
        Err(error) => log::error!("error: {:?}", error),
    };
    Ok(())
}
fn menu_master_avr_v1(master: &Master) -> Result<(), Error> {
    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Select device type")
        .item("d0003_junction_box_minimal_v1")
        .item("d0006_relay14_opto_a_v1")
        .item("d0007_relay14_ssr_a_v2");

    while let Some(result) = menu.interact_opt()? {
        match result {
            0 => menu_master_avr_v1_d0003_junction_box_minimal_v1(master)
                .context("menu_master_avr_v1_d0003_junction_box_minimal_v1")?,
            1 => menu_master_avr_v1_d0006_relay14_opto_a_v1(master)
                .context("menu_master_avr_v1_d0006_relay14_opto_a_v1")?,
            2 => menu_master_avr_v1_d0007_relay14_ssr_a_v2(master)
                .context("menu_master_avr_v1_d0007_relay14_ssr_a_v2")?,
            _ => panic!(),
        };
    }

    Ok(())
}
fn ask_device_serial(
    master: &Master,
    address_device_type: &AddressDeviceType,
) -> Result<AddressSerial, Error> {
    let mut input = dialoguer::Input::<String>::new();
    let input = input
        .with_prompt("Serial (empty for auto-discovery)")
        .allow_empty(true);

    let address_serial = input.interact()?;
    if address_serial.is_empty() {
        let address = master_device_discovery(master).context("master_device_discovery")?;
        if address.device_type() != address_device_type {
            bail!("resolved device type does not match requested device type");
        }
        Ok(*address.serial())
    } else {
        let address_serial =
            AddressSerial::new(address_serial.as_bytes().try_into()?).context("address_serial")?;
        Ok(address_serial)
    }
}
fn menu_master_avr_v1_d0003_junction_box_minimal_v1(master: &Master) -> Result<(), Error> {
    let address_serial =
        ask_device_serial(master, &AddressDeviceType::new_from_ordinal(3).unwrap())
            .context("ask_device_serial")?;
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

            let runner_run = runner_ref.run();
            pin_mut!(runner_run);
            let mut runner_run = runner_run.fuse();

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

                    tokio::time::delay_for(Duration::from_secs(1)).await;
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

                    tokio::time::delay_for(Duration::from_secs(5)).await;
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

            let mut ctrlc = tokio::signal::ctrl_c().boxed().fuse();

            loop {
                select! {
                    _ = runner_run => panic!("runner_run yielded"),
                    _ = leds_runner => panic!("leds_runner"),
                    _ = buzzer_runner => panic!("leds_runner"),
                    _ = properties_remote_in_changed_runner => panic!("properties_remote_in_changed_runner yielded"),
                    _ = ctrlc => break,
                }
            }
        }
        .await;
        runner.finalize().await;
    });
    Ok(())
}
fn menu_master_avr_v1_d0006_relay14_opto_a_v1(master: &Master) -> Result<(), Error> {
    menu_master_avr_v1_common_relay14_common::<
        avr_v1::d0006_relay14_opto_a_v1::hardware::Specification,
    >(master)
}
fn menu_master_avr_v1_d0007_relay14_ssr_a_v2(master: &Master) -> Result<(), Error> {
    menu_master_avr_v1_common_relay14_common::<
        avr_v1::d0007_relay14_ssr_a_v2::hardware::Specification,
    >(master)
}
fn menu_master_avr_v1_common_relay14_common<
    S: avr_v1::common::relay14_common_a::hardware::Specification,
>(
    master: &Master
) -> Result<(), Error> {
    let address_serial =
        ask_device_serial(master, &S::address_device_type()).context("ask_device_serial")?;
    let runner = avr_v1::hardware::runner::Runner::<
        avr_v1::common::relay14_common_a::hardware::Device<S>,
    >::new(master, address_serial);
    execute_on_tokio(async move {
        let runner_ref = &runner;
        async move {
            let avr_v1::common::relay14_common_a::hardware::PropertiesRemote { outputs } =
                runner_ref.properties_remote();

            let runner_run = runner_ref.run();
            pin_mut!(runner_run);
            let mut runner_run = runner_run.fuse();

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
                    tokio::time::delay_for(Duration::from_secs(1)).await;
                }
            };
            pin_mut!(outputs_runner);
            let mut outputs_runner = outputs_runner.fuse();

            let mut ctrlc = tokio::signal::ctrl_c().boxed().fuse();

            loop {
                select! {
                    _ = runner_run => panic!("runner_run yielded"),
                    _ = outputs_runner => panic!("outputs_runner yielded"),
                    _ = ctrlc => break,
                }
            }
        }
        .await;
        runner.finalize().await;
    });
    Ok(())
}
