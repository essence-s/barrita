fn main() {
    unsafe {
        std::env::set_var("SLINT_BACKEND", "winit-femtovg");
    }

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    barrita::run();
}
