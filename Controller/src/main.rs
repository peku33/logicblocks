use futures::future::FutureExt;
use logicblocks_controller::{
    devices::{self, runner::Runner, DeviceContext, Id as DeviceId},
    signals::{exchange::DeviceIdSignalId, Id as SignalId},
    util::select_all_empty::JoinAllEmptyUnit,
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

    let houseblocks_v1_bus_1 = devices::houseblocks::houseblocks_v1::master::Master::new(
        houseblocks_v1_master_descriptors
            .get("DN014CBH")
            .unwrap()
            .clone(),
    )
    .unwrap();

    let devices = hashmap! {
        1 => DeviceContext::new(
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
        2 => DeviceContext::new(
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

        100 => DeviceContext::new(
            "".to_owned(),
            Box::new(
                devices::soft::logic::value::or_default::Device::new(
                    devices::soft::logic::value::or_default::Configuration {
                        default: true,
                    }
                ),
            ),
        ),
        101 => DeviceContext::new(
            "".to_owned(),
            Box::new(
                devices::soft::logic::boolean::slope_a::Device::new(
                    devices::soft::logic::boolean::slope_a::Configuration {
                        edge: devices::soft::logic::boolean::slope_a::Edge::FALLING,
                    }
                ),
            ),
        ),
        102 => DeviceContext::new(
            "".to_owned(),
            Box::new(
                devices::soft::logic::flipflop::rst_a::Device::new(
                    devices::soft::logic::flipflop::rst_a::Configuration {
                        initial_value: false,
                    },
                    None,
                ),
            ),
        ),
    };

    let connections = hashmap! {
        disi(1, 10) => hashset! { disi(100, 0) },
        disi(100, 1) => hashset! { disi(101, 0) },
        disi(101, 1) => hashset! { disi(102, 3) },
        disi(102, 0) => hashset! { disi(2, 0) },
    };

    let device_runner = Runner::new(devices, connections);

    // Web service
    let root_router = MapRouter::new(hashmap! {
        "devices_runner".to_owned() => &device_runner as &(dyn Handler + Sync)
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

    let devices = device_runner.close();

    devices
        .values()
        .map(|device_context| device_context.finalize())
        .collect::<JoinAllEmptyUnit<_>>()
        .await;
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
