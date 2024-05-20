#![feature(async_closure)]
#![allow(clippy::unused_unit)]

use anyhow::{bail, Context, Error};
use clap::{ArgAction, Parser};
use futures::{future::TryFutureExt, join, stream::StreamExt};
use logicblocks_controller::{
    datatypes::ratio::Ratio,
    devices::eaton::mmax_a,
    interfaces::{modbus_rtu, serial},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag, logging,
        runnable::{Exited, Runnable},
    },
};
use std::str::FromStr;
use tokio::signal::ctrl_c;

#[derive(Debug, Clone)]
struct ArgumentsParity(serial::Parity);
impl FromStr for ArgumentsParity {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = match s {
            "None" => serial::Parity::None,
            "Odd" => serial::Parity::Odd,
            "Even" => serial::Parity::Even,
            _ => bail!("unsupported parity. supported values: None, Odd, Even"),
        };

        Ok(Self(inner))
    }
}

#[derive(Debug, Clone)]
struct ArgumentsSpeedSetpoint(Ratio);
impl FromStr for ArgumentsSpeedSetpoint {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = f64::from_str(s).context("from_str")?;
        let value = Ratio::from_f64(value).context("from_f64")?;
        Ok(Self(value))
    }
}

#[derive(Debug, Parser)]
#[clap(name = "devices.eaton.mmax_a")]
struct Arguments {
    ftdi_serial: String,
    baud_rate: usize,
    parity: ArgumentsParity,
    device_address: u8,
    #[clap(subcommand)]
    subcommand: Option<ArgumentsSubcommand>,
}

#[derive(Debug, Parser)]
enum ArgumentsSubcommand {
    Set(ArgumentsSet),
}

#[derive(Debug, Parser)]
#[clap(name = "set")]
struct ArgumentsSet {
    speed_setpoint: ArgumentsSpeedSetpoint,

    #[clap(action = ArgAction::Set)]
    reverse: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!(), true);

    let arguments = Arguments::parse();

    let mut ftdi_global_context = serial::ftdi::Global::new().context("ftdi_global_context")?;
    let ftdi_descriptors = ftdi_global_context
        .find_descriptors()
        .context("find_descriptors")?;

    let ftdi_descriptor = ftdi_descriptors.descriptor_by_serial_or_error(&arguments.ftdi_serial)?;

    let modbus_bus = modbus_rtu::bus::AsyncBus::new(
        ftdi_descriptor.clone(),
        arguments.baud_rate,
        arguments.parity.0,
    );

    let device = mmax_a::hardware::Device::new(&modbus_bus, arguments.device_address);

    let exit_flag_sender = async_flag::Sender::new();

    let device_runner = device.run(exit_flag_sender.receiver());

    if let Some(ArgumentsSubcommand::Set(arguments_set)) = arguments.subcommand {
        let input = mmax_a::hardware::Input {
            control: mmax_a::hardware::InputControl {
                reverse: arguments_set.reverse,
            },
            speed: arguments_set.speed_setpoint.0,
        };
        log::info!("input_set: {:#?}", input);
        device.input_setter().set(input);
    }

    let output_state_receiver_runner = device
        .output_getter()
        .value_stream(true)
        .stream_take_until_exhausted(exit_flag_sender.receiver())
        .for_each(|output_state| async move {
            log::info!("output_state: {:#?}", output_state);
        });

    let exit_flag_runner = ctrl_c()
        .map_ok(|()| {
            log::info!("received exit signal, exiting");
            exit_flag_sender.signal();
        })
        .unwrap_or_else(|error| panic!("ctrl_c error: {:?}", error));

    let _: (Exited, (), ()) = join!(
        device_runner,
        output_state_receiver_runner,
        exit_flag_runner,
    );

    Ok(())
}
