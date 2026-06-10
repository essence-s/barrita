use crate::app::article::ArticleController;
use crate::app::battery::BatteryController;
use crate::app::bluetooth::BluetoothController;
use crate::app::clock::ClockController;
use crate::app::colorize::ColorizeController;
use crate::app::media::popup::PopupController;
use crate::app::network::NetworkController;
use crate::app::screenshot::ScreenshotController;
use crate::app::workspaces::WorkspacesController;
use crate::StatusBarWindow;

pub fn connect_all(window: &StatusBarWindow) {
    ArticleController::connect(window);
    BatteryController::connect(window);
    BluetoothController::connect(window);
    ClockController::connect(window);
    ColorizeController::connect(window);
    NetworkController::connect(window);
    PopupController::connect(window);
    ScreenshotController::connect(window);
    WorkspacesController::connect(window);
}
