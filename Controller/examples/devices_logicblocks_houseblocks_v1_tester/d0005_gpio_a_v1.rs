use super::execute_on_tokio;
use anyhow::{Error, bail};
use futures::{
    future::{FutureExt, join},
    pin_mut, select,
    stream::StreamExt,
};
use itertools::Itertools;
use logicblocks_controller::{
    devices::houseblocks::{
        avr_v1::{
            d0005_gpio_a_v1::hardware::{
                Block1Function, Block1Functions, Block2Function, Block2Functions, Block3Function,
                Block3Functions, Block4Function, Block4Functions, BlockFunctions, Configuration,
                DIGITAL_OUT_COUNT, Device, PropertiesRemote, StatusLedValue,
            },
            hardware::runner::Runner,
        },
        houseblocks_v1::{common::AddressSerial, master::Master},
    },
    util::{async_flag::Sender, runnable::Runnable},
};
use std::time::Duration;
use tokio::signal::ctrl_c;

pub fn menu(
    master: &Master,
    address_serial: AddressSerial,
) -> Result<(), Error> {
    let mut configuration = Configuration {
        block_functions: BlockFunctions {
            block_1_functions: [
                Block1Function::Unused,
                Block1Function::Unused,
                Block1Function::Unused,
                Block1Function::Unused,
            ],
            block_2_functions: [
                Block2Function::Unused,
                Block2Function::Unused,
                Block2Function::Unused,
                Block2Function::Unused,
            ],
            block_3_functions: [
                Block3Function::Unused,
                Block3Function::Unused,
                // line break
            ],
            block_4_functions: [
                Block4Function::Unused,
                Block4Function::Unused,
                Block4Function::Unused,
            ],
        },
    };

    while let Some(option) = dialoguer::Select::new()
        .with_prompt("Select action")
        .default(0)
        .item("Configure")
        .item("Run")
        .interact_opt()?
    {
        match option {
            0 => menu_configuration(&mut configuration)?,
            1 => run(master, address_serial, configuration)?,
            _ => bail!("invalid option"),
        }
    }

    Ok(())
}

fn run(
    master: &Master,
    address_serial: AddressSerial,
    configuration: Configuration,
) -> Result<(), Error> {
    execute_on_tokio(run_inner(master, address_serial, configuration));

    Ok(())
}
async fn run_inner(
    master: &Master,
    address_serial: AddressSerial,
    configuration: Configuration,
) {
    let device = Device::new(configuration);
    let runner = Runner::new(master, address_serial, device);

    let PropertiesRemote {
        ins_changed_waker_remote,
        outs_changed_waker_remote,

        status_led,
        analog_ins,
        digital_ins,
        digital_outs,
        ds18x20s,
    } = runner.device().properties_remote();

    let exit_flag_sender = Sender::new();

    let runner_runner = runner.run(exit_flag_sender.receiver());

    let abort_runner = ctrl_c().then(async |_| {
        exit_flag_sender.signal();
    });

    let status_led_runner = async {
        let mut index = 0;

        loop {
            let r = (index & (1 << 0)) != 0;
            let g = (index & (1 << 1)) != 0;
            let b = (index & (1 << 2)) != 0;
            let status_led_value = StatusLedValue { r, g, b };

            log::info!("status_led: {:?}", status_led_value);
            if status_led.set(status_led_value) {
                outs_changed_waker_remote.wake();
            }

            index += 1;
            index %= 8;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };
    pin_mut!(status_led_runner);
    let mut status_led_runner = status_led_runner.fuse();

    let analog_ins_changed = || {
        let analog_ins = match analog_ins.take_pending() {
            Some(analog_ins) => analog_ins,
            None => return,
        };
        log::info!("analog_ins: {:?}", analog_ins);
    };

    let digital_ins_changed = || {
        let digital_ins = match digital_ins.take_pending() {
            Some(digital_ins) => digital_ins,
            None => return,
        };
        log::info!("digital_ins: {:?}", digital_ins);
    };

    let digital_out_runner = async {
        let mut index = 0;

        loop {
            let mut digital_out_values = [false; DIGITAL_OUT_COUNT];
            digital_out_values[index] = true;

            log::info!("digital_out: {:?}", digital_out_values);
            if digital_outs.set(digital_out_values) {
                outs_changed_waker_remote.wake();
            }

            index += 1;
            index %= DIGITAL_OUT_COUNT;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };
    pin_mut!(digital_out_runner);
    let mut digital_out_runner = digital_out_runner.fuse();

    let ds18x20_changed = || {
        let ds18x20 = match ds18x20s.take_pending() {
            Some(ds18x20) => ds18x20,
            None => return,
        };
        log::info!("ds18x20: {:?}", ds18x20);
    };

    let ins_changed_waker_remote_runner = async {
        futures::stream::once(async {})
            .chain(ins_changed_waker_remote.stream())
            .for_each(async |()| {
                analog_ins_changed();
                digital_ins_changed();
                ds18x20_changed();
            })
            .await;
    };
    pin_mut!(ins_changed_waker_remote_runner);
    let mut ins_changed_waker_remote_runner = ins_changed_waker_remote_runner.fuse();

    select! {
        _ = join(abort_runner, runner_runner).fuse() => {},
        _ = status_led_runner => panic!("status_led_runner"),
        _ = digital_out_runner => panic!("digital_out_runner"),
        _ = ins_changed_waker_remote_runner => panic!("ins_changed_waker_remote_runner"),
    }
}

fn menu_configuration(configuration: &mut Configuration) -> Result<(), Error> {
    while let Some(option) = dialoguer::Select::new()
        .with_prompt("Select configuration option")
        .default(0)
        .item("Block Functions")
        .interact_opt()?
    {
        match option {
            0 => menu_block_functions(&mut configuration.block_functions)?,
            _ => bail!("invalid option"),
        }
    }

    Ok(())
}

fn menu_block_functions(block_functions: &mut BlockFunctions) -> Result<(), Error> {
    while let Some(index) = dialoguer::Select::new()
        .with_prompt("Select block")
        .default(0)
        .item(format!(
            "Block 1 ({})",
            block_functions
                .block_1_functions
                .iter()
                .map(block_1_function_to_string)
                .join(", ")
        ))
        .item(format!(
            "Block 2 ({})",
            block_functions
                .block_2_functions
                .iter()
                .map(block_2_function_to_string)
                .join(", ")
        ))
        .item(format!(
            "Block 3 ({})",
            block_functions
                .block_3_functions
                .iter()
                .map(block_3_function_to_string)
                .join(", ")
        ))
        .item(format!(
            "Block 4 ({})",
            block_functions
                .block_4_functions
                .iter()
                .map(block_4_function_to_string)
                .join(", ")
        ))
        .interact_opt()?
    {
        match index {
            0 => menu_block_1_functions(&mut block_functions.block_1_functions)?,
            1 => menu_block_2_functions(&mut block_functions.block_2_functions)?,
            2 => menu_block_3_functions(&mut block_functions.block_3_functions)?,
            3 => menu_block_4_functions(&mut block_functions.block_4_functions)?,
            _ => bail!("invalid block selected"),
        }
    }

    Ok(())
}

fn menu_block_1_functions(block_1_functions: &mut Block1Functions) -> Result<(), Error> {
    while let Some(index) = dialoguer::Select::new()
        .with_prompt("Select functions for block #1")
        .default(0)
        .items(
            block_1_functions
                .iter()
                .enumerate()
                .map(|(index, block_1_function)| {
                    format!(
                        "Pin #{} ({})",
                        index,
                        block_1_function_to_string(block_1_function)
                    )
                })
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact_opt()?
    {
        block_1_functions[index] = menu_block_1_function(index)?;
    }

    Ok(())
}
fn menu_block_1_function(pin_index: usize) -> Result<Block1Function, Error> {
    let options = [
        Block1Function::Unused,
        Block1Function::AnalogIn,
        Block1Function::DigitalIn,
        Block1Function::DigitalOut,
    ];

    let option_index = dialoguer::Select::new()
        .with_prompt(format!("Select function for block #1 pin #{}", pin_index))
        .default(0)
        .items(
            options
                .iter()
                .map(block_1_function_to_string)
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact()?;

    let block_1_function = options[option_index];

    Ok(block_1_function)
}
fn block_1_function_to_string(block_1_function: &Block1Function) -> &'static str {
    match block_1_function {
        Block1Function::Unused => "Unused",
        Block1Function::AnalogIn => "Analog In",
        Block1Function::DigitalIn => "Digital In",
        Block1Function::DigitalOut => "Digital Out",
    }
}

fn menu_block_2_functions(block_2_functions: &mut Block2Functions) -> Result<(), Error> {
    while let Some(index) = dialoguer::Select::new()
        .with_prompt("Select functions for block #2")
        .default(0)
        .items(
            block_2_functions
                .iter()
                .enumerate()
                .map(|(index, block_2_function)| {
                    format!(
                        "Pin #{} ({})",
                        index,
                        block_2_function_to_string(block_2_function)
                    )
                })
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact_opt()?
    {
        block_2_functions[index] = menu_block_2_function(index)?;
    }

    Ok(())
}
fn menu_block_2_function(pin_index: usize) -> Result<Block2Function, Error> {
    let options = [
        Block2Function::Unused,
        Block2Function::DigitalIn,
        Block2Function::DigitalOut,
        Block2Function::Ds18x20,
    ];

    let option_index = dialoguer::Select::new()
        .with_prompt(format!("Select function for block #2 pin #{}", pin_index))
        .default(0)
        .items(
            options
                .iter()
                .map(block_2_function_to_string)
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact()?;

    let block_2_function = options[option_index];

    Ok(block_2_function)
}
fn block_2_function_to_string(block_2_function: &Block2Function) -> &'static str {
    match block_2_function {
        Block2Function::Unused => "Unused",
        Block2Function::DigitalIn => "Digital In",
        Block2Function::DigitalOut => "Digital Out",
        Block2Function::Ds18x20 => "Ds18x20",
    }
}

fn menu_block_3_functions(block_3_functions: &mut Block3Functions) -> Result<(), Error> {
    while let Some(index) = dialoguer::Select::new()
        .with_prompt("Select functions for block #3")
        .default(0)
        .items(
            block_3_functions
                .iter()
                .enumerate()
                .map(|(index, block_3_function)| {
                    format!(
                        "Pin #{} ({})",
                        index,
                        block_3_function_to_string(block_3_function)
                    )
                })
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact_opt()?
    {
        block_3_functions[index] = menu_block_3_function(index)?;
    }

    Ok(())
}
fn menu_block_3_function(pin_index: usize) -> Result<Block3Function, Error> {
    let options = [Block3Function::Unused, Block3Function::AnalogIn];

    let option_index = dialoguer::Select::new()
        .with_prompt(format!("Select function for block #3 pin #{}", pin_index))
        .default(0)
        .items(
            options
                .iter()
                .map(block_3_function_to_string)
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact()?;

    let block_3_function = options[option_index];

    Ok(block_3_function)
}
fn block_3_function_to_string(block_3_function: &Block3Function) -> &'static str {
    match block_3_function {
        Block3Function::Unused => "Unused",
        Block3Function::AnalogIn => "Analog In",
    }
}

fn menu_block_4_functions(block_4_functions: &mut Block4Functions) -> Result<(), Error> {
    while let Some(index) = dialoguer::Select::new()
        .with_prompt("Select functions for block #4")
        .default(0)
        .items(
            block_4_functions
                .iter()
                .enumerate()
                .map(|(index, block_4_function)| {
                    format!(
                        "Pin #{} ({})",
                        index,
                        block_4_function_to_string(block_4_function)
                    )
                })
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact_opt()?
    {
        block_4_functions[index] = menu_block_4_function(index)?;
    }

    Ok(())
}
fn menu_block_4_function(pin_index: usize) -> Result<Block4Function, Error> {
    let options = [Block4Function::Unused, Block4Function::DigitalOut];

    let option_index = dialoguer::Select::new()
        .with_prompt(format!("Select function for block #4 pin #{}", pin_index))
        .default(0)
        .items(
            options
                .iter()
                .map(block_4_function_to_string)
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .interact()?;

    let block_4_function = options[option_index];

    Ok(block_4_function)
}
fn block_4_function_to_string(block_4_function: &Block4Function) -> &'static str {
    match block_4_function {
        Block4Function::Unused => "Unused",
        Block4Function::DigitalOut => "Digital Out",
    }
}
