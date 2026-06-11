fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    if let Err(e) = barrita::run() {
        log::error!("Error: {e}");
    }
}
