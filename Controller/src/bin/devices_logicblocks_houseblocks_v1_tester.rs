use failure::{err_msg, Error};
use futures::{
    future::{Future, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use logicblocks_controller::devices::houseblocks::{
    avr_v1,
    houseblocks_v1::{
        common::{Address, AddressDeviceType, AddressSerial},
        master::{Master, MasterContext, MasterDescriptor},
    },
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
fn master_device_discovery(master: &Master) -> Result<Address, Error> {
    execute_on_tokio(async move {
        let transaction = master.transaction_device_discovery();
        transaction.await
    })
}
fn menu_master_device_discovery(master: &Master) -> Result<(), Error> {
    match master_device_discovery(master) {
        Ok(address) => println!("address: {}", address),
        Err(error) => println!("error: {}", error),
    };
    Ok(())
}
fn menu_master_avr_v1(master: &Master) -> Result<(), Error> {
    let mut menu = dialoguer::Select::new();
    let menu = menu
        .with_prompt("Select device type")
        .item("d0003_junction_box_minimal_v1");
    // .item("d0006_relay14_opto_a_v1")
    // .item("d0007_relay14_ssr_a_v2");

    while let Some(result) = menu.interact_opt()? {
        match result {
            0 => menu_master_avr_v1_d0003_junction_box_minimal_v1(master),
            // 0 => menu_master_avr_v1_d0006_relay14_opto_a_v1(master),
            // 1 => menu_master_avr_v1_d0007_relay14_ssr_a_v2(master),
            _ => panic!(),
        }?;
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
        let address = master_device_discovery(master)?;
        if address.device_type() != address_device_type {
            return Err(err_msg(
                "resolved device type does not match requested device type",
            ));
        }
        Ok(*address.serial())
    } else {
        let address_serial = AddressSerial::new(address_serial.as_bytes().try_into()?)?;
        Ok(address_serial)
    }
}
fn menu_master_avr_v1_d0003_junction_box_minimal_v1(master: &Master) -> Result<(), Error> {
    let address_serial =
        ask_device_serial(master, &AddressDeviceType::new_from_ordinal(3).unwrap())?;
    let runner = avr_v1::hardware::runner::Runner::<
        avr_v1::d0003_junction_box_minimal_v1::hardware::Device,
    >::new(master, address_serial);
    execute_on_tokio(async move {
        let runner_ref = &runner;
        async move {
            let avr_v1::d0003_junction_box_minimal_v1::hardware::RemoteProperties {
                mut keys,
                leds,
                buzzer,
                mut temperature,
            } = runner_ref.remote_properties();

            let runner_run = runner_ref.run();
            pin_mut!(runner_run);
            let mut runner_run = runner_run.fuse();

            let leds_runner = async {
                let mut led_index = 0;

                loop {
                    led_index += 1;
                    led_index %= avr_v1::d0003_junction_box_minimal_v1::hardware::LED_COUNT;

                    let mut led_values =
                        [false; avr_v1::d0003_junction_box_minimal_v1::hardware::LED_COUNT];
                    led_values[led_index] = true;

                    log::info!("setting leds: {:?}", led_values);
                    leds.set(led_values);
                    tokio::time::delay_for(Duration::from_secs(1)).await;
                }
            };
            pin_mut!(leds_runner);
            let mut leds_runner = leds_runner.fuse();

            let buzzer_test_runner = async {
                loop {
                    log::info!("pushing buzzer");
                    buzzer.set(Duration::from_millis(125));
                    tokio::time::delay_for(Duration::from_secs(5)).await;
                }
            };
            pin_mut!(buzzer_test_runner);
            let mut buzzer_test_runner = buzzer_test_runner.fuse();

            let mut ctrlc = tokio::signal::ctrl_c().boxed().fuse();

            loop {
                select! {
                    _ = runner_run => panic!("runner_run yielded"),
                    keys = keys.select_next_some() => {
                        log::info!("keys: {:?}", keys);
                    },
                    _ = leds_runner => panic!("leds_runner yielded"),
                    _ = buzzer_test_runner => panic!("buzzer_test_runner yielded"),
                    temperature = temperature.select_next_some() => {
                        match temperature {
                            Some(temperature) => log::info!("temperature: {}", temperature),
                            None => log::warn!("temperature: None (Error)"),
                        }
                    }
                    _ = ctrlc => break,
                }
            }
        }
        .await;
        runner.finalize().await;
    });
    Ok(())
}
