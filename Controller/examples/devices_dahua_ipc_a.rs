#![feature(async_closure)]
#![allow(clippy::unused_unit)]

use anyhow::{Context, Error};
use clap::{ArgAction, Parser};
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use http::uri::Authority;
use logicblocks_controller::{
    devices::dahua::ipc_a::hardware::{
        api::Api,
        configurator::{
            AudioMutationDetection, Configuration, Configurator, Grid22x18, MotionDetection,
            MotionDetectionRegion, Percentage, SceneMovedDetection, Sensitivity,
            SmartMotionDetection, SmartMotionDetectionSensitivity,
        },
        event_stream::Manager,
    },
    util::logging,
};
use tokio::signal::ctrl_c;

#[derive(Debug, Parser)]
#[clap(name = "devices.dahua.ipc_a")]
struct Arguments {
    host: Authority,
    admin_password: String,

    #[clap(subcommand)]
    subcommand: Option<ArgumentsSubcommand>,
}

#[derive(Debug, Parser)]
enum ArgumentsSubcommand {
    Configure(CommandConfigure),
}

#[derive(Debug, Parser)]
#[clap(name = "configure")]
struct CommandConfigure {
    device_name: String,
    device_id: u8,
    shared_user_password: String,
    #[clap(action = ArgAction::Set)]
    video_upside_down: bool,
    channel_title: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!());

    let arguments = Arguments::parse();

    let api = Api::new(arguments.host, arguments.admin_password);

    if let Some(ArgumentsSubcommand::Configure(command_configure)) = arguments.subcommand {
        let mut configurator = Configurator::connect(&api).await.context("connect")?;
        log::info!("basic_device_info: {:?}", configurator.basic_device_info());
        log::info!("starting configuration");
        configurator
            .configure(Configuration {
                device_id: command_configure.device_id,
                device_name: command_configure.device_name,
                shared_user_password: command_configure.shared_user_password,
                video_upside_down: command_configure.video_upside_down,
                channel_title: Some(command_configure.channel_title),
                privacy_mask: None,
                motion_detection: Some(MotionDetection::single(MotionDetectionRegion {
                    grid: Grid22x18::full(),
                    name: "Motion Detection".to_owned(),
                    sensitivity: Percentage::new(75).unwrap(),
                    threshold: Percentage::new(10).unwrap(),
                })),
                smart_motion_detection: Some(SmartMotionDetection {
                    human: true,
                    vehicle: true,
                    sensitivity: SmartMotionDetectionSensitivity::Medium,
                }),
                scene_moved_detection: Some(SceneMovedDetection {
                    sensitivity: Sensitivity::new(5).unwrap(),
                }),
                audio_mutation_detection: Some(AudioMutationDetection {
                    sensitivity: Percentage::new(50).unwrap(),
                }),
            })
            .await
            .context("configure")?;
        log::info!("configuration completed");
    } else {
        let basic_device_info = api
            .validate_basic_device_info()
            .await
            .context("validate_basic_device_info")?;
        log::info!("basic_device_info: {:?}", basic_device_info);
    }

    let event_stream_manager = Manager::new(&api);

    let event_stream_manager_runner = event_stream_manager.run();
    pin_mut!(event_stream_manager_runner);
    let mut event_stream_manager_runner = event_stream_manager_runner.fuse();

    let event_stream_manager_receiver_runner = tokio_stream::wrappers::WatchStream::new(
        event_stream_manager.receiver(),
    )
    .for_each(async move |events| {
        log::info!("events: {:?}", events);
    });
    pin_mut!(event_stream_manager_receiver_runner);
    let mut event_stream_manager_receiver_runner = event_stream_manager_receiver_runner.fuse();

    select! {
        _ = ctrl_c().fuse() => (),
        _ = event_stream_manager_runner => panic!("event_stream_manager_runner yielded"),
        _ = event_stream_manager_receiver_runner => panic!("event_stream_manager_receiver_runner yielded"),
    }

    Ok(())
}
