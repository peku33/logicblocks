#![feature(async_closure)]
#![allow(clippy::unused_unit)]

use anyhow::{Context, Error};
use clap::Clap;
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

#[derive(Clap, Debug)]
#[clap(name = "devices.hikvision.ds2cd2x32x_x")]
struct Arguments {
    host: Authority,
    admin_password: String,

    #[clap(subcommand)]
    subcommand: Option<ArgumentsSubcommand>,
}

#[derive(Clap, Debug)]
enum ArgumentsSubcommand {
    Configure(CommandConfigure),
}

#[derive(Clap, Debug)]
#[clap(name = "configure")]
struct CommandConfigure {
    device_name: String,
    device_id: u8,
    shared_user_password: String,
    #[clap(parse(try_from_str))]
    video_upside_down: bool,
    overlay_text: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    logging::configure();

    let arguments = Arguments::parse();

    let api = Api::new(arguments.host, arguments.admin_password);

    let basic_device_info = api
        .validate_basic_device_info()
        .await
        .context("validate_basic_device_info")?;
    log::info!("basic_device_info: {:?}", basic_device_info);

    if let Some(ArgumentsSubcommand::Configure(command_configure)) = arguments.subcommand {
        log::info!("starting configuration");
        let mut configurator = Configurator::new(&api);
        configurator
            .configure(Configuration {
                device_name: command_configure.device_name,
                device_id: command_configure.device_id,
                shared_user_password: command_configure.shared_user_password,
                video_upside_down: command_configure.video_upside_down,
                overlay_text: command_configure.overlay_text,
                privacy_mask: None,
                motion_detection: Some(
                    MotionDetection::new(vec![MotionDetectionRegion {
                        region: RegionSquare::full(),
                        sensitivity: Percentage::new(50).unwrap(),
                        object_size: Percentage::new(0).unwrap(),
                    }])
                    .unwrap(),
                ),
                field_detection: None,
                line_detection: None,
            })
            .await
            .context("configure")?;
        log::info!("configuration completed");
    }

    let event_stream_manager = Manager::new(&api);

    let event_stream_manager_runner = event_stream_manager.run();
    pin_mut!(event_stream_manager_runner);
    let mut event_stream_manager_runner = event_stream_manager_runner.fuse();

    let mut event_stream_manager_receiver =
        tokio_stream::wrappers::WatchStream::new(event_stream_manager.receiver());
    let event_stream_manager_receiver_runner =
        event_stream_manager_receiver
            .by_ref()
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
