use slint::ComponentHandle;

pub struct BluetoothController;

impl BluetoothController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let adapter = window.global::<crate::BluetoothAdapter>();
        adapter.on_bluetooth_clicked(|| {
            log::info!("[bluetooth] clicked");
        });
    }
}
