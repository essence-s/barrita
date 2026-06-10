use slint::ComponentHandle;

pub struct ScreenshotController;

impl ScreenshotController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let adapter = window.global::<crate::ScreenshotAdapter>();
        adapter.on_screenshot_clicked(|| {
            log::info!("[screenshot] screenshot clicked");
        });
    }
}
