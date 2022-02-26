use super::common::execute_on_tokio;
use anyhow::Error;
use futures::{
    future::{join, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use logicblocks_controller::{
    devices::houseblocks::{
        avr_v1::{
            d0003_junction_box_minimal_v1::hardware::{Device, PropertiesRemote, LED_COUNT},
            hardware::runner::Runner,
        },
        houseblocks_v1::{common::AddressSerial, master::Master},
    },
    util::async_flag::Sender,
};
use std::time::Duration;
use tokio::signal::ctrl_c;

pub fn run(
    master: &Master,
    address_serial: AddressSerial,
) -> Result<(), Error> {
    execute_on_tokio(run_inner(master, address_serial));

    Ok(())
}

async fn run_inner(
    master: &Master,
    address_serial: AddressSerial,
) {
    let device = Device::new();
    let runner = Runner::new(master, device, address_serial);

    let PropertiesRemote {
        keys,
        leds,
        buzzer,
        ds18x20,
    } = runner.properties_remote();

    let exit_flag_sender = Sender::new();

    let runner_runner = runner.run(exit_flag_sender.receiver());

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
            let mut led_values = [false; LED_COUNT];
            led_values[led_index] = true;

            log::info!("leds: {:?}", led_values);
            if leds.set(led_values) {
                runner.properties_remote_out_change_waker_wake();
            }

            led_index += 1;
            led_index %= LED_COUNT;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };
    pin_mut!(leds_runner);
    let mut leds_runner = leds_runner.fuse();

    let buzzer_runner = async {
        const DURATION: Duration = Duration::from_millis(125);
        loop {
            log::info!("buzzer: {:?}", DURATION);
            if buzzer.push(DURATION) {
                runner.properties_remote_out_change_waker_wake();
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    };
    pin_mut!(buzzer_runner);
    let mut buzzer_runner = buzzer_runner.fuse();

    let ds18x20_changed = || {
        let ds18x20 = match ds18x20.take_pending() {
            Some(ds18x20) => ds18x20,
            None => return,
        };

        match ds18x20 {
            Some(ds18x20) => log::info!("ds18x20: {:?}", ds18x20),
            None => log::warn!("ds18x20: None (Error)"),
        }
    };

    let properties_remote_in_changed_runner = async {
        runner
            .properties_remote_in_change_waker_receiver()
            .by_ref()
            .for_each(|()| async move {
                keys_changed();
                ds18x20_changed();
            })
            .await;
    };
    pin_mut!(properties_remote_in_changed_runner);
    let mut properties_remote_in_changed_runner = properties_remote_in_changed_runner.fuse();

    select! {
        _ = join(abort_runner, runner_runner).fuse() => {},
        _ = leds_runner => panic!("leds_runner"),
        _ = buzzer_runner => panic!("leds_runner"),
        _ = properties_remote_in_changed_runner => panic!("properties_remote_in_changed_runner yielded"),
    }
}
