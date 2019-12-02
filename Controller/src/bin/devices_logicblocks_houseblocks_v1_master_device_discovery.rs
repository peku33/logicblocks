extern crate logicblocks_controller;

use logicblocks_controller::devices::logicblocks::houseblocks_v1::master::{Master, MasterContext};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let master_context = MasterContext::new()?;
    let master_descriptors_by_serial_number = master_context
        .find_master_descriptors()?
        .into_iter()
        .map(|master_descriptor| {
            (
                master_descriptor
                    .get_serial_number()
                    .clone()
                    .into_string()
                    .unwrap(),
                master_descriptor,
            )
        })
        .collect::<HashMap<_, _>>();

    let serial_number = env::args().nth(1);
    let master_descriptor = match serial_number {
        Some(serial_number) => master_descriptors_by_serial_number
            .get(&serial_number)
            .map(|master_descriptor| master_descriptor.clone()),
        None => None,
    };
    let master_descriptor = match master_descriptor {
        Some(master_descriptor) => master_descriptor,
        None => {
            println!(
                "master_descriptor not found, available: {:#?}",
                master_descriptors_by_serial_number
            );
            return Ok(());
        }
    };

    let mut master = Master::new(master_descriptor)?;
    let transaction = master.transaction_device_discovery(Duration::from_secs(1));
    let transaction_result = transaction.await;
    println!("transaction_result: {:#?}", transaction_result);

    return Ok(());
}
