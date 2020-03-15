use logicblocks_controller::devices::logicblocks::houseblocks_v1::master::{Master, MasterContext};
use std::collections::HashMap;
use std::env;
use std::error::Error;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let master_context = MasterContext::new()?;
    let master_descriptors_by_serial_number = master_context
        .find_master_descriptors()?
        .into_iter()
        .map(|master_descriptor| {
            (
                master_descriptor
                    .serial_number
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
            .cloned(),
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

    let master = Master::new(master_descriptor)?;
    let transaction = master.transaction_device_discovery();
    let address = transaction.await;
    println!("address: {:?}", address);

    return Ok(());
}
