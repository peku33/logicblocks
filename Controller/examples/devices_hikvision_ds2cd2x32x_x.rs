#![allow(clippy::unused_unit)]

use anyhow::{Context, Error};
use clap::{ArgAction, Parser};
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use http::uri::Authority;
use logicblocks_controller::{
    devices::hikvision::ds2cd2x32x_x::hardware::{
        api::Api,
        configurator::{
            Configuration, Configurator, MotionDetection, MotionDetectionRegion, Percentage,
            RegionSquare,
        },
        event_stream::Manager,
    },
    util::logging,
};
use tokio::signal::ctrl_c;

#[derive(Debug, Parser)]
#[clap(name = "devices.hikvision.ds2cd2x32x_x")]
struct Arguments {
    host: Authority,
    admin_password: String,

    #[clap(subcommand)]
    subcommand: ArgumentsSubcommand,
}

#[derive(Debug, Parser)]
enum ArgumentsSubcommand {
    EventStream,
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
    overlay_text: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure(module_path!(), true);

    let arguments = Arguments::parse();

    let api = Api::new(arguments.host, arguments.admin_password);

    match arguments.subcommand {
        ArgumentsSubcommand::EventStream => {
            let basic_device_info = api
                .validate_basic_device_info()
                .await
                .context("validate_basic_device_info")?;
            log::info!("basic_device_info: {basic_device_info:?}");

            let event_stream_manager = Manager::new(&api);

            let event_stream_manager_runner = event_stream_manager.run();
            pin_mut!(event_stream_manager_runner);
            let mut event_stream_manager_runner = event_stream_manager_runner.fuse();

            let event_stream_manager_receiver_runner = tokio_stream::wrappers::WatchStream::new(
                event_stream_manager.receiver(),
            )
            .for_each(async |events| {
                log::info!("events: {events:?}");
            });
            pin_mut!(event_stream_manager_receiver_runner);
            let mut event_stream_manager_receiver_runner =
                event_stream_manager_receiver_runner.fuse();

            select! {
                _ = ctrl_c().fuse() => (),
                _ = event_stream_manager_runner => panic!("event_stream_manager_runner yielded"),
                _ = event_stream_manager_receiver_runner => panic!("event_stream_manager_receiver_runner yielded"),
            }
        }
        ArgumentsSubcommand::Configure(command_configure) => {
            let mut configurator = Configurator::connect(&api).await.context("connect")?;
            log::info!("basic_device_info: {:?}", configurator.basic_device_info());
            log::info!("capabilities: {:?}", configurator.capabilities());
            log::info!("starting configuration");
            configurator
                .configure(Configuration {
                    device_name: command_configure.device_name,
                    device_id: command_configure.device_id,
                    shared_user_password: command_configure.shared_user_password,
                    video_upside_down: command_configure.video_upside_down,
                    overlay_text: command_configure.overlay_text,
                    privacy_mask: None,
                    motion_detection: Some(
                        MotionDetection::new(
                            vec![MotionDetectionRegion {
                                region: RegionSquare::full(),
                                sensitivity: Percentage::new(50).unwrap(),
                                object_size: Percentage::new(0).unwrap(),
                            }]
                            .into_boxed_slice(),
                        )
                        .unwrap(),
                    ),
                    field_detection: None,
                    line_detection: None,
                })
                .await
                .context("configure")?;
            log::info!("configuration completed");
        }
    }

    Ok(())
}
