use super::{
    devices::{
        helpers::{Devices, Signals},
        runner::Runner,
    },
    web::{
        root_service::RootService,
        server,
        uri_cursor::{map_router::MapRouter, Handler},
    },
};
use anyhow::{Context, Error};
use maplit::hashmap;
use tokio::signal::ctrl_c;

pub async fn run(
    devices: Devices<'_>,
    signals: Signals,
) -> Result<(), Error> {
    let devices = devices.into_devices();
    let signals = signals.into_signals();

    // devices runner
    let device_runner = Runner::new(devices, &signals);

    // web service
    let root_router = MapRouter::new(hashmap! {
        "devices-runner".to_owned() => &device_runner as &(dyn Handler + Sync)
    });
    let root_service = RootService::new(&root_router);
    let server_runner = server::ServerRunner::new("0.0.0.0:8080".parse().unwrap(), &root_service);

    // wait for exit signal
    log::info!("application started, awaiting exit signal");
    ctrl_c().await.context("ctrlc")?;
    log::info!("received exit signal, closing application");

    // teardown
    server_runner.finalize().await;
    device_runner.finalize().await;

    // bye bye
    Ok(())
}
