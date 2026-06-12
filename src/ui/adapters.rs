use crate::app::article::ArticleController;
use crate::app::battery::BatteryController;
use crate::app::bluetooth::BluetoothController;
use crate::app::clock::ClockController;
use crate::app::colorize::ColorizeController;
use crate::app::media::MediaController;
use crate::app::media::popup::PopupController;
use crate::app::network::NetworkController;
use crate::app::screenshot::ScreenshotController;
use crate::app::workspaces::WorkspacesController;
use crate::StatusBarWindow;
use spell_framework::wayland_adapter::WinHandle;

pub fn connect_all(window: &StatusBarWindow, popup_handler: WinHandle) {
    ArticleController::connect(window);
    BatteryController::connect(window);
    BluetoothController::connect(window);
    ClockController::connect(window);
    ColorizeController::connect(window);
    MediaController::connect(window);
    NetworkController::connect(window);
    PopupController::connect(window, popup_handler);
    ScreenshotController::connect(window);
    WorkspacesController::connect(window);
}
