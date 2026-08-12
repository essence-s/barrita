use crate::app::article::ArticleController;
use crate::app::battery::BatteryController;
use crate::app::bluetooth::BluetoothController;
use crate::app::clock::ClockController;
use crate::app::colorize::ColorizeController;
use crate::app::control_center::ControlCenterController;
use crate::app::media::MediaController;
use crate::app::network::NetworkController;
use crate::app::screenshot::ScreenshotController;
use crate::app::tray::TrayController;
use crate::app::workspaces::WorkspacesController;
use crate::StatusBarWindow;
use slint_layer_shell::wayland_adapter::WinHandle;

pub fn connect_all(window: &StatusBarWindow, ctrl_handler: WinHandle, tray_popup_handler: WinHandle, popup_weak: slint::Weak<crate::TrayPopup>) {
    ArticleController::connect(window);
    BatteryController::connect(window);
    BluetoothController::connect(window);
    ClockController::connect(window);
    ColorizeController::connect(window);
    ControlCenterController::connect(window, ctrl_handler.clone());
    MediaController::connect(window);
    NetworkController::connect(window);
    ScreenshotController::connect(window);
    TrayController::connect(window, tray_popup_handler, popup_weak);
    WorkspacesController::connect(window);
}
