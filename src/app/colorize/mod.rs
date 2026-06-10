use slint::ComponentHandle;

pub struct ColorizeController;

impl ColorizeController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let adapter = window.global::<crate::ColorizeAdapter>();
        adapter.on_colorize_clicked(|| {
            log::info!("[colorize] clicked");
        });
    }
}
