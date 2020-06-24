use futures::FutureExt;
use logicblocks_controller::{
    datatypes, devices,
    logic::{
        device::Device,
        runner::{DeviceIdSignalId, Runner},
    },
    web::{
        root_service::RootService,
        server,
        uri_cursor::{map_router::MapRouter, Handler},
    },
};
use maplit::{hashmap, hashset};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Debug)
        .init();

    // Drivers, etc
    let houseblocks_v1_master_context =
        devices::houseblocks::houseblocks_v1::master::MasterContext::new().unwrap();
    let houseblocks_v1_master_descriptors = houseblocks_v1_master_context
        .find_master_descriptors()
        .unwrap();
    let houseblocks_v1_master_descriptors = houseblocks_v1_master_descriptors
        .into_iter()
        .map(|master_descriptor| {
            (
                master_descriptor
                    .serial_number
                    .to_string_lossy()
                    .to_string(),
                master_descriptor,
            )
        })
        .collect::<HashMap<_, _>>();

    let houseblocks_v1_bus_0 = devices::houseblocks::houseblocks_v1::master::Master::new(
        houseblocks_v1_master_descriptors
            .get("DN014CBH")
            .unwrap()
            .clone(),
    )
    .unwrap();

    let devices = hashmap! {
        0 => Box::new(
            devices::houseblocks::avr_v1::logic::Runner::<
                devices::houseblocks::avr_v1::d0003_junction_box_minimal_v1::logic::Device,
            >::new(
                &houseblocks_v1_bus_0,
                devices::houseblocks::houseblocks_v1::common::AddressSerial::new(*b"82651052").unwrap(),
            )
        ) as Box<dyn Device>,

        10 => Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        11 => Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        12 => Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        13 => Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        14 => Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        15 => Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,

        100 => Box::new(devices::soft::debug_state::Device::<datatypes::boolean::Boolean>::new("Key #0".to_owned())) as Box<dyn Device>,
        101 => Box::new(devices::soft::debug_state::Device::<datatypes::boolean::Boolean>::new("Key #1".to_owned())) as Box<dyn Device>,
        102 => Box::new(devices::soft::debug_state::Device::<datatypes::boolean::Boolean>::new("Key #2".to_owned())) as Box<dyn Device>,
        103 => Box::new(devices::soft::debug_state::Device::<datatypes::boolean::Boolean>::new("Key #3".to_owned())) as Box<dyn Device>,
        104 => Box::new(devices::soft::debug_state::Device::<datatypes::boolean::Boolean>::new("Key #4".to_owned())) as Box<dyn Device>,
        105 => Box::new(devices::soft::debug_state::Device::<datatypes::boolean::Boolean>::new("Key #5".to_owned())) as Box<dyn Device>,

        20 => Box::new(devices::soft::rst_a::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        21 => Box::new(devices::soft::rst_a::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        22 => Box::new(devices::soft::rst_a::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        23 => Box::new(devices::soft::rst_a::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        24 => Box::new(devices::soft::rst_a::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,
        25 => Box::new(devices::soft::rst_a::Device::new(datatypes::boolean::Boolean::from(false))) as Box<dyn Device>,

        // TODO: Buzzer

        40 => Box::new(devices::soft::debug_state::Device::<Option<datatypes::temperature::Temperature>>::new("Temperature".to_owned())) as Box<dyn Device>,
    };

    let connections = hashmap! {
        DeviceIdSignalId::new(0, 10) => hashset! { DeviceIdSignalId::new(10, 1), },
        DeviceIdSignalId::new(0, 11) => hashset! { DeviceIdSignalId::new(11, 1), },
        DeviceIdSignalId::new(0, 12) => hashset! { DeviceIdSignalId::new(12, 1), },
        DeviceIdSignalId::new(0, 13) => hashset! { DeviceIdSignalId::new(13, 1), },
        DeviceIdSignalId::new(0, 14) => hashset! { DeviceIdSignalId::new(14, 1), },
        DeviceIdSignalId::new(0, 15) => hashset! { DeviceIdSignalId::new(15, 1), },

        DeviceIdSignalId::new(10, 0) => hashset! { DeviceIdSignalId::new(100, 0), },
        DeviceIdSignalId::new(11, 0) => hashset! { DeviceIdSignalId::new(101, 0), },
        DeviceIdSignalId::new(12, 0) => hashset! { DeviceIdSignalId::new(102, 0), },
        DeviceIdSignalId::new(13, 0) => hashset! { DeviceIdSignalId::new(103, 0), },
        DeviceIdSignalId::new(14, 0) => hashset! { DeviceIdSignalId::new(104, 0), },
        DeviceIdSignalId::new(15, 0) => hashset! { DeviceIdSignalId::new(105, 0), },

        DeviceIdSignalId::new(20, 0) => hashset! { DeviceIdSignalId::new(0, 20), },
        DeviceIdSignalId::new(21, 0) => hashset! { DeviceIdSignalId::new(0, 21), },
        DeviceIdSignalId::new(22, 0) => hashset! { DeviceIdSignalId::new(0, 22), },
        DeviceIdSignalId::new(23, 0) => hashset! { DeviceIdSignalId::new(0, 23), },
        DeviceIdSignalId::new(24, 0) => hashset! { DeviceIdSignalId::new(0, 24), },
        DeviceIdSignalId::new(25, 0) => hashset! { DeviceIdSignalId::new(0, 25), },

        // TODO: Buzzer

        DeviceIdSignalId::new(0, 40) => hashset! { DeviceIdSignalId::new(40, 0), },
    };

    let device_runner = Runner::new(devices, connections);

    // Web service
    let root_router = MapRouter::new(hashmap! {
        "device_runner".to_owned() => &device_runner as &(dyn Handler + Sync)
    });

    let root_service = RootService::new(&root_router);

    let server = server::serve("0.0.0.0:8080".parse().unwrap(), &root_service);

    futures::select! {
        _ = tokio::signal::ctrl_c().fuse() => (),
        _ = device_runner.run().fuse() => {
            panic!("device_runner.run() yielded");
        },
        _ = server.fuse() => {
            panic!("server yielded")
        }
    }

    device_runner.finalize().await;
}
