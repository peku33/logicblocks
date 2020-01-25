use futures::FutureExt;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Trace)
        .init();

    futures::select! {
        _ = tokio::signal::ctrl_c().fuse() => (),
        main_error = main_async().fuse() => panic!("{}", main_error),
    }
}
async fn main_async() -> failure::Error {
    // Device Pool
    let (device_pool_api_bridge_sender, device_pool_api_bridge_receiver) =
        logicblocks_controller::web::uri_cursor::local_bridge::channel();
    let device_pool_local_set = tokio::task::LocalSet::new();
    let device_pool_local_set_future = device_pool_local_set.run_until(async move {
        let device_pool = logicblocks_controller::devices::pool::Pool::new();

        let device_pool_api_bridge_receiver_future =
            device_pool_api_bridge_receiver.attach_run(&device_pool);
        let device_pool_future = device_pool.run();

        futures::select! {
            _ = device_pool_api_bridge_receiver_future.fuse() => failure::err_msg("device_pool_api_bridge_receiver_future exited"),
            device_pool_future_error = device_pool_future.fuse() => device_pool_future_error,
        }
    });

    // API Handler
    let mut api_handler =
        logicblocks_controller::web::uri_cursor::next_item_map::NextItemMap::default();

    // API Routes
    api_handler.set("device_pool".to_owned(), &device_pool_api_bridge_sender);

    // Web server
    let web_root_service =
        logicblocks_controller::web::root_service::RootService::new(&api_handler);
    let server_future = logicblocks_controller::web::server::serve(
        std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 8080),
        &web_root_service,
    );

    // Global runner
    futures::select! {
        device_pool_local_set_future_error = device_pool_local_set_future.fuse() => device_pool_local_set_future_error,
        server_future_error = server_future.fuse() => server_future_error,
    }
}
