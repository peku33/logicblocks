#![feature(async_closure)]
#![allow(clippy::unused_unit)]

use anyhow::{anyhow, bail, Context, Error};
use clap::Clap;
use futures::{future::TryFutureExt, join, stream::StreamExt};
use logicblocks_controller::{
    datatypes::ratio::Ratio,
    devices::eaton::mmax_a,
    interfaces::{modbus_rtu, serial},
    util::{async_flag, logging, runtime::Exited},
};
use std::{collections::HashMap, convert::TryFrom, str::FromStr};
use tokio::signal::ctrl_c;

#[derive(Debug)]
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

#[derive(Debug)]
struct ArgumentsSpeedSetpoint(Ratio);
impl FromStr for ArgumentsSpeedSetpoint {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = f64::from_str(s)?;
        let value = Ratio::try_from(value)?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clap)]
#[clap(name = "devices.eaton.mmax_a")]
struct Arguments {
    ftdi_serial: String,
    baud_rate: usize,
    parity: ArgumentsParity,
    device_address: u8,
    #[clap(subcommand)]
    subcommand: Option<ArgumentsSubcommand>,
}

#[derive(Debug, Clap)]
enum ArgumentsSubcommand {
    Set(ArgumentsSet),
}

#[derive(Debug, Clap)]
#[clap(name = "set")]
struct ArgumentsSet {
    speed_setpoint: ArgumentsSpeedSetpoint,

    #[clap(parse(try_from_str))]
    reverse: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!());

    let arguments = Arguments::parse();

    let mut ftdi_global_context = serial::ftdi::Global::new().context("ftdi_global_context")?;
    let ftdi_descriptors = ftdi_global_context
        .find_descriptors()
        .context("find_descriptors")?;
    let ftdi_descriptors = ftdi_descriptors
        .into_iter()
        .map(|descriptor| {
            (
                descriptor.serial_number.to_string_lossy().to_string(),
                descriptor,
            )
        })
        .collect::<HashMap<_, _>>();

    let ftdi_descriptor = ftdi_descriptors
        .get(&arguments.ftdi_serial)
        .ok_or_else(|| anyhow!("descriptor not found on available descriptor list"))?
        .clone();

    let modbus_bus =
        modbus_rtu::bus::AsyncBus::new(ftdi_descriptor, arguments.baud_rate, arguments.parity.0);

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

    let mut output_state_receiver = device
        .output_getter()
        .value_stream(true)
        .take_until(exit_flag_sender.receiver());
    let output_state_receiver_runner =
        output_state_receiver
            .by_ref()
            .for_each(async move |output_state| {
                log::info!("output_state: {:#?}", output_state);
            });

    let exit_flag_runner = ctrl_c()
        .map_ok(|()| {
            log::info!("received exit signal, exiting");
            exit_flag_sender.signal();
            Exited
        })
        .unwrap_or_else(|error| panic!("ctrl_c error: {:?}", error));

    let _: (Exited, (), Exited) = join!(
        device_runner,
        output_state_receiver_runner,
        exit_flag_runner,
    );

    assert!(output_state_receiver.is_stopped());

    Ok(())
}
