#![feature(async_closure)]
#![allow(clippy::unused_unit)]

use anyhow::{Context, Error};
use clap::{App, Arg, SubCommand};
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    logging::configure();

    let arguments = App::new("devices.hikvision.ds2cd2x32x_x Runner")
        .arg(Arg::with_name("host").required(true).validator(|value| {
            value
                .parse::<Authority>()
                .map(|_| ())
                .map_err(|error| error.to_string())
        }))
        .arg(Arg::with_name("admin_password").required(true))
        .subcommand(
            SubCommand::with_name("configure")
                .arg(Arg::with_name("device_name").required(true))
                .arg(
                    Arg::with_name("device_id")
                        .required(true)
                        .validator(|value| {
                            value
                                .parse::<u8>()
                                .map(|_| ())
                                .map_err(|error| error.to_string())
                        }),
                )
                .arg(Arg::with_name("overlay_text").required(true))
                .arg(Arg::with_name("shared_user_password").required(true)),
        )
        .get_matches();

    let host: Authority = arguments.value_of("host").unwrap().parse().unwrap();
    let admin_password = arguments.value_of("admin_password").unwrap();

    let api = Api::new(host, admin_password.to_owned());

    let basic_device_info = api
        .validate_basic_device_info()
        .await
        .context("validate_basic_device_info")?;
    log::info!("basic_device_info: {:?}", basic_device_info);

    if let Some(arguments) = arguments.subcommand_matches("configure") {
        log::info!("starting configuration");
        let mut configurator = Configurator::new(&api);
        configurator
            .configure(Configuration {
                device_name: arguments.value_of("device_name").unwrap().to_owned(),
                device_id: arguments.value_of("device_id").unwrap().parse().unwrap(),
                overlay_text: arguments.value_of("overlay_text").unwrap().to_owned(),
                shared_user_password: arguments
                    .value_of("shared_user_password")
                    .unwrap()
                    .to_owned(),
                privacy_mask: None,
                motion_detection: Some(
                    MotionDetection::new(vec![MotionDetectionRegion {
                        region: RegionSquare::full(),
                        sensitivity: Percentage::new(100).unwrap(),
                        object_size: Percentage::new(0).unwrap(),
                    }])
                    .unwrap(),
                ),
                field_detection: None,
                line_detection: None,
            })
            .await?;
        log::info!("configuration completed");
    }

    let event_stream_manager = Manager::new(&api);

    let event_stream_manager_run = event_stream_manager.run();
    pin_mut!(event_stream_manager_run);
    let mut event_stream_manager_run = event_stream_manager_run.fuse();

    let mut event_stream_manager_receiver = event_stream_manager.receiver();
    let event_stream_manager_receiver_runner =
        event_stream_manager_receiver
            .by_ref()
            .for_each(async move |events| {
                log::info!("events: {:?}", events);
            });
    pin_mut!(event_stream_manager_receiver_runner);
    let mut event_stream_manager_receiver_runner = event_stream_manager_receiver_runner.fuse();

    select! {
        _ = tokio::signal::ctrl_c().fuse() => (),
        _ = event_stream_manager_run => panic!("event_stream_manager_run yielded"),
        _ = event_stream_manager_receiver_runner => panic!("event_stream_manager_receiver_runner yielded"),
    }

    Ok(())
}
