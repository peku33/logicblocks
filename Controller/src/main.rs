use futures::FutureExt;
use logicblocks_controller::{
    logic::{
        device::Device,
        device_provider::DeviceProvider,
        device_provider_static::DeviceProviderStatic,
        device_providers::DeviceProviderIdDeviceId,
        devices::soft::{debug_state, rst_a},
        runner::{DeviceIdSignalId, Runner},
        signal_values::Bool,
    },
    web::{
        root_service::RootService,
        server,
        uri_cursor::{map_router::MapRouter, Handler},
    },
};
use maplit::{hashmap, hashset};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Trace)
        .init();

    let device_provider_static = DeviceProviderStatic::new(vec![
        // 0
        Box::new(rst_a::Device::new(Bool::new(false))) as Box<dyn Device>,
        // 1
        Box::new(debug_state::Device::<Bool>::new(
            "Debug device #1".to_owned(),
        )) as Box<dyn Device>,
        // 2
        Box::new(debug_state::Device::<Bool>::new(
            "Debug device #2".to_owned(),
        )) as Box<dyn Device>,
    ]);

    let device_providers = maplit::hashmap! {
        0 => &device_provider_static as &dyn DeviceProvider,
    };

    let connections = hashmap! {
        DeviceIdSignalId::new(DeviceProviderIdDeviceId::new(0, 0), 3) => hashset! {
            DeviceIdSignalId::new(DeviceProviderIdDeviceId::new(0, 1), 0),
            DeviceIdSignalId::new(DeviceProviderIdDeviceId::new(0, 2), 0),
        },
    };

    let device_runner = Runner::new(device_providers, connections);

    // Web service
    let root_router = MapRouter::new(hashmap! {
        "device_runner".to_owned() => &device_runner as &(dyn Handler + Sync)
    });

    let root_service = RootService::new(&root_router);

    let server = server::serve("127.0.0.1:8080".parse().unwrap(), &root_service);

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
