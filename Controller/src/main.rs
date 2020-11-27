use logicblocks_controller::{
    devices::{self, runner::Runner, DeviceHandler, Id as DeviceId},
    interfaces,
    signals::{exchange::DeviceIdSignalId, Id as SignalId},
    util::logging,
    web::{
        root_service::RootService,
        server,
        uri_cursor::{map_router::MapRouter, Handler},
    },
};
use maplit::{hashmap, hashset};
use std::collections::HashMap;

fn main() {
    logging::configure();

    // Drivers, etc
    let mut ftdi_global_context = interfaces::serial::ftdi::Global::new().unwrap();
    let ftdi_descriptors = ftdi_global_context.find_descriptors().unwrap();
    let ftdi_descriptors = ftdi_descriptors
        .into_iter()
        .map(|descriptor| {
            (
                descriptor.serial_number.to_string_lossy().to_string(),
                descriptor,
            )
        })
        .collect::<HashMap<_, _>>();

    let houseblocks_v1_bus_1 = devices::houseblocks::houseblocks_v1::master::Master::new(
        ftdi_descriptors.get("DN014CBH").unwrap().clone(),
    );

    let devices = hashmap! {
        1 => DeviceHandler::new(
            "d0003_junction_box_minimal_v1".to_owned(),
            Box::new(
                devices::houseblocks::avr_v1::logic::Runner::<
                    devices::houseblocks::avr_v1::d0003_junction_box_minimal_v1::logic::Device,
                >::new(
                    &houseblocks_v1_bus_1,
                    devices::houseblocks::houseblocks_v1::common::AddressSerial::new(*b"72031321").unwrap(),
                )
            ),
        ),
        2 => DeviceHandler::new(
            "d0007_relay14_ssr_a_v2".to_owned(),
            Box::new(
                devices::houseblocks::avr_v1::logic::Runner::<
                    devices::houseblocks::avr_v1::d0007_relay14_ssr_a_v2::logic::Device,
                >::new(
                    &houseblocks_v1_bus_1,
                    devices::houseblocks::houseblocks_v1::common::AddressSerial::new(*b"44467979").unwrap(),
                )
            ),
        ),

        100 => DeviceHandler::new(
            "".to_owned(),
            Box::new(
                devices::soft::logic::boolean::slope_a::Device::new(
                    devices::soft::logic::boolean::slope_a::Configuration {
                        edge: devices::soft::logic::boolean::slope_a::Edge::Falling,
                    }
                ),
            ),
        ),
        101 => DeviceHandler::new(
            "".to_owned(),
            Box::new(
                devices::soft::logic::flipflop::rst_a::Device::new(
                    devices::soft::logic::flipflop::rst_a::Configuration {
                        initial_value: false,
                    },
                ),
            ),
        ),
    };

    let connections = hashmap! {
        disi(1, 10) => hashset! { disi(100, 0) },
        disi(100, 1) => hashset! { disi(101, 3) },
        disi(101, 0) => hashset! { disi(2, 0) },
    };

    let device_runner = Runner::new(devices, connections);

    // Web service
    let root_router = MapRouter::new(hashmap! {
        "devices-runner".to_owned() => &device_runner as &(dyn Handler + Sync)
    });
    let root_service = RootService::new(&root_router);
    let server_runner = server::ServerRunner::new("0.0.0.0:8080".parse().unwrap(), &root_service);

    // Wait for exit signal
    // TODO: Make it a bit smarter, without using runtime, lol
    let mut runtime = tokio::runtime::Builder::new()
        .enable_all()
        .basic_scheduler()
        .build()
        .unwrap();
    runtime.block_on(tokio::signal::ctrl_c()).unwrap();

    // This is done automatically, for debugging purposes
    drop(server_runner);
    drop(device_runner);
}

fn disi(
    device_id: DeviceId,
    signal_id: SignalId,
) -> DeviceIdSignalId {
    DeviceIdSignalId {
        device_id,
        signal_id,
    }
}
