use crate::StatusBarWindow;
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Weak};

pub fn configure_backend() {
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("SLINT_BACKEND", "winit-skia");
    }
}

pub fn setup_window(app: &StatusBarWindow) {
    let win = app.window();
    win.set_size(PhysicalSize::new(1920, 32));
    win.set_position(PhysicalPosition::new(0, 0));
}

pub fn get_window_weak(app: &StatusBarWindow) -> Weak<StatusBarWindow> {
    app.as_weak()
}