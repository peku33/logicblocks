pub fn configure(root_module: &str) {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("logicblocks_controller", log::LevelFilter::Trace)
        .filter_module(root_module, log::LevelFilter::Trace)
        .init();
}
