pub mod appbar;
pub mod config;
pub mod position_monitor;
pub mod init;

pub use appbar::{get_window_position, install_appbar_window_proc};
pub use config::{AppBarEdge, StatusBarConfig};

pub fn init_statusbar(config: &StatusBarConfig, hwnd: isize) {
    println!(
        "[statusbar] init_statusbar: height={}, edge={:?}",
        config.height, config.edge
    );

    install_appbar_window_proc(hwnd);

    appbar::force_window_position(hwnd, 0, 0, 1366, config.height);

    position_monitor::start_position_monitor(hwnd, config.height);
}