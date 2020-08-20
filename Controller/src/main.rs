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

    let devices = hashmap! {
        0 => DeviceContext::new(
            "RST".to_owned(),
            Box::new(devices::soft::rst_a::Device::new(datatypes::boolean::Boolean::from(false))),
        ),
    };

    let connections = hashmap! {};

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
