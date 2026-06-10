use crate::{MediaPopupWindow, StatusBarWindow};
use slint::{ComponentHandle, PhysicalPosition, Weak, WindowPosition};
use std::sync::Mutex;

static POPUP_VISIBLE: Mutex<bool> = Mutex::new(false);
static POPUP_WEAK: Mutex<Option<Weak<MediaPopupWindow>>> = Mutex::new(None);

pub struct PopupController;

impl PopupController {
    pub fn connect(window: &StatusBarWindow) {
        let window_weak = window.as_weak();
        window.on_popup_toggle(move || {
            Self::toggle(&window_weak);
        });
    }

    fn toggle(app_weak: &Weak<StatusBarWindow>) {
        let is_visible = *POPUP_VISIBLE.lock().unwrap();

        if is_visible {
            Self::hide();
        } else {
            Self::show(app_weak);
        }
    }

    fn hide() {
        let mut popup_weak_guard = POPUP_WEAK.lock().unwrap();
        if let Some(weak) = popup_weak_guard.take() {
            if let Some(popup) = weak.upgrade() {
                let _ = popup.hide();
            }
        }
        *POPUP_VISIBLE.lock().unwrap() = false;
    }

    fn show(app_weak: &Weak<StatusBarWindow>) {
        if let Some(app) = app_weak.upgrade() {
            let pos = app.window().position();
            let size = app.window().size();
            let popup_x = pos.x;
            let popup_y = pos.y + size.height as i32;

            let popup = MediaPopupWindow::new().unwrap();
            popup.window().set_position(WindowPosition::Physical(
                PhysicalPosition::new(popup_x, popup_y),
            ));
            let _ = popup.show();

            *POPUP_WEAK.lock().unwrap() = Some(popup.as_weak());
            *POPUP_VISIBLE.lock().unwrap() = true;
        }
    }
}
