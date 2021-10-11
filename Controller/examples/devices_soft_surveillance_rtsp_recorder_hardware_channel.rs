#![feature(async_closure)]
#![allow(clippy::unused_unit)]

use anyhow::Error;
use clap::Clap;
use futures::{future::TryFutureExt, join, stream::StreamExt};
use logicblocks_controller::{
    datatypes::{ipc_rtsp_url::IpcRtspUrl, ratio::Ratio},
    devices::soft::surveillance::rtsp_recorder::hardware::channel::Channel,
    util::{
        async_flag, logging,
        runtime::{Exited, Runnable},
    },
};
use rand::Rng;
use std::{convert::TryInto, path::PathBuf, time::Duration};
use tokio::{fs, signal::ctrl_c};

#[derive(Debug, Clap)]
#[clap(name = "devices.soft.surveillance.rtsp_recorder.hardware.channel")]
struct Arguments {
    rtsp_url: IpcRtspUrl,
    temporary_storage_directory: PathBuf,
}

const SEGMENT_TIME: Duration = Duration::from_secs(30);
const DETECTION_CHANGE_INTERVAL: Duration = Duration::from_secs(20);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!());

    let arguments = Arguments::parse();

    let exit_flag_sender = async_flag::Sender::new();

    // channel
    let channel = Channel::new(
        Some(arguments.rtsp_url),
        SEGMENT_TIME,
        arguments.temporary_storage_directory,
        (0.50).try_into().unwrap(),
    );
    let channel_runner = channel.run(exit_flag_sender.receiver());

    // delete recorded clips
    let mut channel_segment_receiver_lease = channel.channel_segment_receiver_lease();
    let segment_deleter_runner = channel_segment_receiver_lease
        .by_ref()
        // TODO: convert take_until to something like "take_until_non_empty_async_flag"
        .take_until(exit_flag_sender.receiver())
        .for_each(async move |channel_segment| {
            log::info!("received segment: {:?}", channel_segment);
            fs::remove_file(channel_segment.segment.path).await.unwrap();
        });

    // set random detection level for testing
    let detection_level_set_runner_channel = &channel;
    let detection_level_set_runner = tokio_stream::wrappers::IntervalStream::new(
        tokio::time::interval(DETECTION_CHANGE_INTERVAL),
    )
    // TODO: convert take_until to something like "take_until_non_empty_async_flag"
    .take_until(exit_flag_sender.receiver())
    .for_each(async move |_| {
        let mut rng = rand::thread_rng();
        let detection_level: Option<Ratio> = if rng.gen_bool(0.7) {
            let ratio_f64: f64 = rng.gen_range(0.0..1.0);
            Some(ratio_f64.try_into().unwrap())
        } else {
            None
        };

        log::info!(
            "detection_level_set_runner: setting detection_level = {:?}",
            detection_level
        );

        detection_level_set_runner_channel.detection_level_set(detection_level);
    });

    // wait for exit flag
    let exit_flag_runner = ctrl_c()
        .map_ok(|()| {
            log::info!("received exit signal, exiting");
            exit_flag_sender.signal();
            Exited
        })
        .unwrap_or_else(|error| panic!("ctrl_c error: {:?}", error));

    // orchestrate all
    let _: (Exited, (), (), Exited) = join!(
        channel_runner,
        segment_deleter_runner,
        detection_level_set_runner,
        exit_flag_runner,
    );

    Ok(())
}
