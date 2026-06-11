use crate::StatusBarWindow;
use spell_framework::wayland_adapter::WinHandle;

pub struct PopupController;

impl PopupController {
    pub fn connect(window: &StatusBarWindow, handler: WinHandle) {
        window.on_popup_toggle(move || {
            handler.toggle();
        });
    }
}
