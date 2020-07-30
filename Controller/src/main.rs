use futures::FutureExt;
use logicblocks_controller::{
    datatypes, devices,
    logic::{
        device::DeviceContext,
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
            .get("DN014CBC")
            .unwrap()
            .clone(),
    )
    .unwrap();

    let devices = hashmap! {
        0 => DeviceContext::new(
            Box::new(
                devices::houseblocks::avr_v1::logic::Runner::<
                    devices::houseblocks::avr_v1::d0003_junction_box_minimal_v1::logic::Device,
                >::new(
                    &houseblocks_v1_bus_0,
                    devices::houseblocks::houseblocks_v1::common::AddressSerial::new(*b"82651052").unwrap(),
                )
            ),
        ),
        1 => DeviceContext::new(
            Box::new(
                devices::houseblocks::avr_v1::logic::Runner::<
                    devices::houseblocks::avr_v1::d0007_relay14_ssr_a_v2::logic::Device,
                >::new(
                    &houseblocks_v1_bus_0,
                    devices::houseblocks::houseblocks_v1::common::AddressSerial::new(*b"44467979").unwrap(),
                )
            ),
        ),

        // Button unwrapper
        10 => DeviceContext::new(
            Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(true))),
        ),

        // Button inverter
        11 => DeviceContext::new(
            Box::new(devices::soft::boolean_invert::Device::new()),
        ),

        // Inverter unwrapper
        12 => DeviceContext::new(
            Box::new(devices::soft::state_unwrap::Device::new(datatypes::boolean::Boolean::from(false))),
        ),

        // Button delay controller
        13 => DeviceContext::new(
            Box::new(devices::soft::button_controller_a::Device::new()),
        ),

        // Debug outputs
        14 => DeviceContext::new(
            Box::new(devices::soft::debug_event::Device::<datatypes::void::Void>::new("Short press".to_owned())),
        ),
        15 => DeviceContext::new(
            Box::new(devices::soft::debug_event::Device::<datatypes::void::Void>::new("Long press".to_owned())),
        ),
        16 => DeviceContext::new(
            Box::new(devices::soft::debug_state::Device::<datatypes::boolean::Boolean>::new("Button press value".to_owned())),
        ),
    };

    let connections = hashmap! {
        DeviceIdSignalId::new(0, 10) => hashset! { DeviceIdSignalId::new(10, 0), },
        DeviceIdSignalId::new(10, 1) => hashset! { DeviceIdSignalId::new(11, 0), },
        DeviceIdSignalId::new(11, 1) => hashset! { DeviceIdSignalId::new(12, 0), },
        DeviceIdSignalId::new(12, 1) => hashset! { DeviceIdSignalId::new(13, 0), DeviceIdSignalId::new(16, 0), },
        DeviceIdSignalId::new(13, 1) => hashset! { DeviceIdSignalId::new(14, 0), },
        DeviceIdSignalId::new(13, 2) => hashset! { DeviceIdSignalId::new(15, 0), },
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
