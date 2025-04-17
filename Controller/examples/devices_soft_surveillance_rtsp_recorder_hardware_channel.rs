#![allow(clippy::unused_unit)]

use anyhow::Error;
use clap::Parser;
use futures::{future::TryFutureExt, join, stream::StreamExt};
use logicblocks_controller::{
    datatypes::{ipc_rtsp_url::IpcRtspUrl, ratio::Ratio},
    devices::soft::surveillance::rtsp_recorder::hardware::channel::Channel,
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag, logging,
        runnable::{Exited, Runnable},
    },
};
use rand::Rng;
use std::{path::PathBuf, time::Duration};
use tokio::{fs, signal::ctrl_c};

#[derive(Debug, Parser)]
#[clap(name = "devices.soft.surveillance.rtsp_recorder.hardware.channel")]
struct Arguments {
    rtsp_url: IpcRtspUrl,
    temporary_storage_directory: PathBuf,
}

const SEGMENT_TIME: Duration = Duration::from_secs(30);
const DETECTION_CHANGE_INTERVAL: Duration = Duration::from_secs(20);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!(), true);

    let arguments = Arguments::parse();

    let exit_flag_sender = async_flag::Sender::new();

    // channel
    let channel = Channel::new(
        Some(arguments.rtsp_url),
        SEGMENT_TIME,
        arguments.temporary_storage_directory,
        Ratio::from_f64(0.50).unwrap(),
    );
    let channel_runner = channel.run(exit_flag_sender.receiver());

    // delete recorded clips
    let mut channel_segment_receiver = channel.channel_segment_receiver_borrow_mut();
    let segment_deleter_runner = channel_segment_receiver
        .by_ref()
        .stream_take_until_exhausted(exit_flag_sender.receiver())
        .for_each(async |channel_segment| {
            log::info!("received segment: {:?}", channel_segment);
            fs::remove_file(channel_segment.segment.path).await.unwrap();
        });

    // set random detection level for testing
    let detection_level_set_runner_channel = &channel;
    let detection_level_set_runner = tokio_stream::wrappers::IntervalStream::new(
        tokio::time::interval(DETECTION_CHANGE_INTERVAL),
    )
    .stream_take_until_exhausted(exit_flag_sender.receiver())
    .for_each(async |_| {
        let mut rng = rand::rng();
        let detection_level: Option<Ratio> = if rng.random_bool(0.7) {
            let ratio_f64: f64 = rng.random_range(0.0..1.0);
            Some(Ratio::from_f64(ratio_f64).unwrap())
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
        })
        .unwrap_or_else(|error| panic!("ctrl_c error: {:?}", error));

    // orchestrate all
    let _: (Exited, (), (), ()) = join!(
        channel_runner,
        segment_deleter_runner,
        detection_level_set_runner,
        exit_flag_runner,
    );

    Ok(())
}
