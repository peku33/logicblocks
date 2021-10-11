#![feature(async_closure)]
#![allow(clippy::unused_unit)]

use anyhow::Error;
use clap::Clap;
use futures::{
    channel::mpsc,
    future::{FutureExt, TryFutureExt},
    join,
    stream::StreamExt,
};
use logicblocks_controller::{
    datatypes::ipc_rtsp_url::IpcRtspUrl,
    devices::soft::surveillance::rtsp_recorder::hardware::recorder::Recorder,
    util::{
        async_flag,
        fs::move_file,
        logging,
        runtime::{Exited, Runnable},
    },
};
use std::{path::PathBuf, time::Duration};
use tokio::signal::ctrl_c;

#[derive(Debug, Clap)]
#[clap(name = "devices.soft.surveillance.rtsp_recorder.hardware.recorder")]
struct Arguments {
    rtsp_url: IpcRtspUrl,
    segment_time_seconds: u64,
    temporary_storage_directory: PathBuf,
    persistent_storage_directory: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!());

    let arguments = Arguments::parse();

    let exit_flag_sender = async_flag::Sender::new();

    // set up recorder
    let (segment_sender, mut segment_receiver) = mpsc::unbounded();
    let recorder = Recorder::new(
        Some(arguments.rtsp_url),
        Duration::from_secs(arguments.segment_time_seconds),
        arguments.temporary_storage_directory,
        segment_sender,
    );
    let recorder_runner = recorder.run(exit_flag_sender.receiver());

    // forwarder to target directory
    let forwarder_runner_persistent_storage_directory = &arguments.persistent_storage_directory;
    let forwarder_runner = segment_receiver
        .by_ref()
        // TODO: convert take_until to something like "take_until_non_empty_async_flag"
        .take_until(exit_flag_sender.receiver())
        .for_each(async move |segment| {
            let target_path = forwarder_runner_persistent_storage_directory
                .join(segment.path.file_name().unwrap());
            log::info!(
                "received segment: {:?}, moving to {:?}",
                segment,
                target_path
            );
            move_file(segment.path, target_path).await.unwrap();
        })
        .then(async move |()| Exited);

    // exit flag runner
    let exit_flag_runner = ctrl_c()
        .map_ok(|()| {
            log::info!("received exit signal, exiting");
            exit_flag_sender.signal();
            Exited
        })
        .unwrap_or_else(|error| panic!("ctrl_c error: {:?}", error));

    // orchestrate all
    let _: (Exited, Exited, Exited) = join!(recorder_runner, forwarder_runner, exit_flag_runner);

    Ok(())
}
