use super::common::execute_on_tokio;
use anyhow::Error;
use futures::{
    future::{FutureExt, join},
    pin_mut, select,
    stream::StreamExt,
};
use logicblocks_controller::{
    devices::houseblocks::{
        avr_v1::{
            devices::d0003_junction_box_minimal_v1::hardware::{
                Device, LEDS_COUNT, PropertiesRemote,
            },
            hardware::runner::Runner,
        },
        houseblocks_v1::{common::AddressSerial, master::Master},
    },
    util::{async_flag::Sender, runnable::Runnable},
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
    let runner = Runner::new(master, address_serial, device);

    let PropertiesRemote {
        ins_changed_waker_remote,
        outs_changed_waker_remote,

        keys,
        leds,
        buzzer,
        ds18x20,
    } = runner.device().properties_remote();

    let exit_flag_sender = Sender::new();

    let runner_runner = runner.run(exit_flag_sender.receiver());

    let abort_runner = ctrl_c().then(async |_| {
        exit_flag_sender.signal();
    });

    let keys_changed = || {
        let keys = match keys.take_pending() {
            Some(keys) => keys,
            None => return,
        };
        log::info!("keys: {keys:?}");
    };

    let leds_runner = async {
        let mut led_index = 0;

        loop {
            let mut led_values = [false; LEDS_COUNT];
            led_values[led_index] = true;

            log::info!("leds: {led_values:?}");
            if leds.set(led_values) {
                outs_changed_waker_remote.wake();
            }

            led_index += 1;
            led_index %= LEDS_COUNT;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    .fuse();
    pin_mut!(leds_runner);

    let buzzer_runner = async {
        const DURATION: Duration = Duration::from_millis(125);
        loop {
            log::info!("buzzer: {DURATION:?}");
            if buzzer.push(DURATION) {
                outs_changed_waker_remote.wake();
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
    .fuse();
    pin_mut!(buzzer_runner);

    let ds18x20_changed = || {
        let ds18x20 = match ds18x20.take_pending() {
            Some(ds18x20) => ds18x20,
            None => return,
        };

        match ds18x20 {
            Some(ds18x20) => log::info!("ds18x20: {ds18x20:?}"),
            None => log::warn!("ds18x20: None (Error)"),
        }
    };

    let ins_changed_waker_remote_runner = async {
        futures::stream::once(async {})
            .chain(ins_changed_waker_remote.stream())
            .for_each(async |()| {
                keys_changed();
                ds18x20_changed();
            })
            .await;
    }
    .fuse();
    pin_mut!(ins_changed_waker_remote_runner);

    select! {
        _ = join(abort_runner, runner_runner).fuse() => {},
        _ = leds_runner => panic!("leds_runner"),
        _ = buzzer_runner => panic!("leds_runner"),
        _ = ins_changed_waker_remote_runner => panic!("ins_changed_waker_remote_runner"),
    }
}
