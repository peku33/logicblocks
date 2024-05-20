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
            d0002_reed_switch_v1::hardware::{Device, PropertiesRemote},
            hardware::runner::Runner,
        },
        houseblocks_v1::{common::AddressSerial, master::Master},
    },
    util::{async_flag::Sender, runnable::Runnable},
};
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

        inputs,
    } = runner.device().properties_remote();

    let exit_flag_sender = Sender::new();

    let runner_runner = runner.run(exit_flag_sender.receiver());

    let abort_runner = ctrl_c().then(|_| async {
        exit_flag_sender.signal();
    });

    let inputs_changed = || {
        let inputs = match inputs.take_pending() {
            Some(inputs) => inputs,
            None => return,
        };
        log::info!("inputs: {:?}", inputs);
    };

    let ins_changed_waker_remote_runner = async {
        futures::stream::once(async {})
            .chain(ins_changed_waker_remote.stream())
            .for_each(|()| async {
                inputs_changed();
            })
            .await;
    };
    pin_mut!(ins_changed_waker_remote_runner);
    let mut ins_changed_waker_remote_runner = ins_changed_waker_remote_runner.fuse();

    select! {
        _ = join(abort_runner, runner_runner).fuse() => {},
        _ = ins_changed_waker_remote_runner => panic!("ins_changed_waker_remote_runner"),
    }
}
