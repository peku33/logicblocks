extern crate logicblocks_controller;

use futures::stream::StreamExt;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Trace)
        .init();

    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();

    runtime.spawn(main_result());

    log::info!("logicblocks_controller starting");
    let ctrlc = tokio_net::signal::ctrl_c().unwrap().into_future();
    let _ = runtime.block_on(ctrlc);
    log::info!("logicblocks_controller closed");
}

async fn main_result() -> () {
    #[cfg(target_os = "linux")]
    test_1().await.unwrap();
}
