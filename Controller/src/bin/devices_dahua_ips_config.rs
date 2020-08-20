use failure::Error;
// use logicblocks_controller::devices::dahua::ipc::api::{Client, SaneDefaultsConfig};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Trace)
        .init();

    // let args = std::env::args().collect::<Vec<_>>();
    // if args.len() != 6 {
    //     eprintln!(
    //         "Usage: <host> <admin_password> <device_name> <shared_user_password> [video_overlay]"
    //     );
    // }

    // let host = args[1].parse()?;
    // let admin_password = args[2].to_owned();

    // let device_name = args[3].to_owned();
    // let shared_user_password = args[4].to_owned();
    // let video_overlay = match args[5].as_str() {
    //     "" => None,
    //     video_overlay => Some(video_overlay.to_owned()),
    // };

    // let client = Client::new(host, admin_password);

    // client
    //     .sane_defaults(&SaneDefaultsConfig {
    //         device_name,
    //         shared_user_password,
    //         video_overlay,
    //     })
    //     .await?;

    return Ok(());
}
