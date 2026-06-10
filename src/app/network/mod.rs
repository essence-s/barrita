use slint::ComponentHandle;

pub mod types;

pub struct NetworkController;

impl NetworkController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let adapter = window.global::<crate::NetworkAdapter>();
        adapter.on_network_clicked(|| {
            log::info!("[network] clicked");
        });
    }
}
