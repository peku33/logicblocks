extern crate logicblocks_controller;

use futures::future::FutureExt;
use futures::select;
use futures::stream::StreamExt;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Trace)
        .init();

    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();

    #[cfg(target_os = "linux")]
    runtime.spawn(main_async());

    log::info!("logicblocks_controller starting");
    let ctrlc = tokio_net::signal::ctrl_c().unwrap().into_future();
    let _ = runtime.block_on(ctrlc);
    log::info!("logicblocks_controller closed");
}

#[cfg(target_os = "linux")]
async fn main_async() -> () {
    main_async_result().await.unwrap();
    return ();
}
#[cfg(target_os = "linux")]
async fn main_async_result() -> Result<(), failure::Error> {
    // HouseBlocks v1
    let houseblocks_v1_master_context =
        logicblocks_controller::devices::logicblocks::houseblocks_v1::master::MasterContext::new()?;
    let houseblocks_v1_masters_by_serial = houseblocks_v1_master_context.find_master_descriptors()?.iter().map(|master_descriptor| (
        master_descriptor.serial_number.clone().into_string().unwrap(),
        std::cell::RefCell::new(logicblocks_controller::devices::logicblocks::houseblocks_v1::master::Master::new(master_descriptor.clone()).unwrap()),
    )).collect::<std::collections::HashMap<
        String,
        std::cell::RefCell<logicblocks_controller::devices::logicblocks::houseblocks_v1::master::Master
    >>>();

    // Device Pool
    let mut device_pool = logicblocks_controller::devices::pool::Pool::new();

    // HouseBlocks v1 devices

    // Web server
    let mut web_router_map_items = std::collections::HashMap::<
        &str,
        &dyn logicblocks_controller::web::router::uri_cursor::Handler,
    >::new();
    web_router_map_items.insert("device_pool", &device_pool);

    let web_router_map =
        logicblocks_controller::web::router::uri_cursor::Map::new(web_router_map_items);

    let web_router_root =
        logicblocks_controller::web::router::uri_cursor::Root::new(&web_router_map);

    let (web_handler_async_sender, web_handler_async_receiver) =
        logicblocks_controller::web::handler_async(&web_router_root);

    let web_server_error_future = logicblocks_controller::web::server::run_server(
        std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 8080),
        &web_handler_async_sender,
        Some("*"),
    );

    // Global error
    let error = select! {
        device_pool_error = device_pool.run().boxed_local().fuse() => device_pool_error,
        web_handler_async_receiver_error = web_handler_async_receiver.run().boxed_local().fuse() => web_handler_async_receiver_error,
        web_server_error = web_server_error_future.boxed().fuse() => web_server_error,
    };

    return Err(error);
}
