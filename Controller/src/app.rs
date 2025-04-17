use super::{
    devices::{
        helpers::{Devices, Signals},
        runner::Runner,
    },
    web::{
        root_service::RootService,
        server,
        uri_cursor::{Handler, map_router::MapRouter},
    },
};
use crate::gui::dashboards;
use anyhow::{Context, Error};
use maplit::hashmap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::signal::ctrl_c;

pub async fn run(
    devices: Devices<'_>,
    signals: Signals,
    dashboards: dashboards::Dashboard,
    bind_custom: Option<SocketAddrV4>,
) -> Result<(), Error> {
    let device_wrappers_by_id = devices.into_device_wrappers_by_id();
    let connections_requested = signals.into_connections_requested();

    // devices runner
    let device_runner =
        Runner::new(device_wrappers_by_id, &connections_requested).context("new")?;

    // web service
    let gui_router = MapRouter::new(hashmap! {
        "dashboards".to_owned() => &dashboards as &(dyn Handler + Sync),
    });
    let root_router = MapRouter::new(hashmap! {
        "devices-runner".to_owned() => &device_runner as &(dyn Handler + Sync),
        "gui".to_owned() => &gui_router as &(dyn Handler + Sync),
    });
    let root_service = RootService::new(&root_router);
    let server_runner = server::RunnerOwned::new(
        SocketAddr::V4(
            bind_custom.unwrap_or_else(|| SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8080)),
        ),
        &root_service,
    );

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
