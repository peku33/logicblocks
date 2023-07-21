pub fn configure(
    root_module: &str,
    tracing: bool,
) {
    let level = if tracing {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Debug
    };

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", level)
        .filter_module(root_module, level)
        .init();
}
