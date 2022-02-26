use super::execute_on_tokio;
use anyhow::Error;
use futures::{
    future::{join, FutureExt},
    pin_mut, select,
};
use logicblocks_controller::{
    devices::houseblocks::{
        avr_v1::{
            common::relay14_common_a::hardware::{
                Device, PropertiesRemote, Specification, OUTPUT_COUNT,
            },
            hardware::runner::Runner,
        },
        houseblocks_v1::{common::AddressSerial, master::Master},
    },
    util::async_flag::Sender,
};
use std::time::Duration;
use tokio::signal::ctrl_c;

pub fn run<S: Specification>(
    master: &Master,
    address_serial: AddressSerial,
) -> Result<(), Error> {
    execute_on_tokio(run_inner::<S>(master, address_serial));

    Ok(())
}

async fn run_inner<S: Specification>(
    master: &Master,
    address_serial: AddressSerial,
) {
    let device = Device::<S>::new();
    let runner = Runner::new(master, device, address_serial);

    let PropertiesRemote { outputs } = runner.properties_remote();

    let exit_flag_sender = Sender::new();

    let runner_runner = runner.run(exit_flag_sender.receiver());

    let abort_runner = ctrl_c().then(async move |_| {
        exit_flag_sender.signal();
    });

    let outputs_runner = async {
        let mut output_index = 0;

        loop {
            let mut output_values = [false; OUTPUT_COUNT];
            output_values[output_index] = true;

            log::info!("outputs: {:?}", output_values);
            if outputs.set(output_values) {
                runner.properties_remote_out_change_waker_wake();
            }

            output_index += 1;
            output_index %= OUTPUT_COUNT;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };
    pin_mut!(outputs_runner);
    let mut outputs_runner = outputs_runner.fuse();

    select! {
        _ = join(abort_runner, runner_runner).fuse() => {},
        _ = outputs_runner => panic!("outputs_runner yielded"),
    }
}
