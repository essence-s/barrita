slint::include_modules!();

mod app;
mod config;
// mod tray;  // TODO: tray icon en Wayland (StatusNotifierItem)
mod ui;

pub fn run() {
    let _cfg = config::load_or_create_config();

    // TODO: tray icon en Wayland (StatusNotifierItem)
    // let _tray = tray::init_tray().map(|t| Box::leak(Box::new(t)));

    let window = StatusBarWindow::new().unwrap();
    ui::adapters::connect_all(&window);
    window.run().unwrap();
}
