pub mod types;

pub struct BatteryController;

impl BatteryController {
    pub fn connect(_window: &crate::StatusBarWindow) {
        log::info!("[battery] controller connected (placeholder)");
    }
}
