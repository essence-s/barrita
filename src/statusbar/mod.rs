pub mod app_bar;
pub mod config;
pub mod position_monitor;

pub use app_bar::get_window_position;
pub use config::{AppBarEdge, StatusBarConfig};

pub fn init_statusbar(config: &StatusBarConfig, hwnd: isize) {
    println!(
        "[statusbar] init_statusbar: height={}, edge={:?}",
        config.height, config.edge
    );

    app_bar::install_appbar_window_proc(hwnd);

    app_bar::force_window_position(hwnd, 0, 0, 1366, config.height);

    position_monitor::start_position_monitor(hwnd, config.height);
}
