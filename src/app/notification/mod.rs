use slint::ComponentHandle;
use slint_layer_shell::wayland_adapter::WinHandle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[allow(dead_code)]
pub const SEVERITY_INFO: i32 = 0;
pub const SEVERITY_WARNING: i32 = 1;
#[allow(dead_code)]
pub const SEVERITY_ERROR: i32 = 2;

const WINDOW_W: i32 = 300;
const WINDOW_H: i32 = 500;
const MAX_TOASTS: usize = 5;
const DEFAULT_DURATION_MS: i32 = 5000;

thread_local! {
    static NEXT_ID: Cell<i32> = const { Cell::new(0) };
    static QUEUE: RefCell<Vec<crate::ToastData>> = const { RefCell::new(Vec::new()) };
    static TIMERS: RefCell<Vec<(i32, slint::Timer)>> = const { RefCell::new(Vec::new()) };
    static CTX: RefCell<Option<(WinHandle, slint::Weak<crate::NotificationPopup>)>> =
        const { RefCell::new(None) };
}

fn next_id() -> i32 {
    NEXT_ID.with(|c| {
        let id = c.get() + 1;
        c.set(id);
        id
    })
}

fn rebuild_model(popup_weak: &slint::Weak<crate::NotificationPopup>) {
    let toasts = QUEUE.with(|q| q.borrow().clone());
    let model = Rc::new(slint::VecModel::from(toasts));
    if let Some(popup) = popup_weak.upgrade() {
        popup
            .global::<crate::NotificationAdapter>()
            .set_toasts(model.into());
    }
}

fn update_input_region(
    popup_handler: &WinHandle,
    popup_weak: &slint::Weak<crate::NotificationPopup>,
) {
    popup_handler.subtract_input_region(0, 0, WINDOW_W, WINDOW_H);
    let height = popup_weak
        .upgrade()
        .map(|p| p.get_content_height() as i32 + 2)
        .unwrap_or(0);
    if height > 0 {
        popup_handler.add_input_region(0, 0, WINDOW_W, height);
    }
}

fn remove_toast(
    id: i32,
    popup_handler: &WinHandle,
    popup_weak: &slint::Weak<crate::NotificationPopup>,
) {
    TIMERS.with(|t| {
        let mut timers = t.borrow_mut();
        timers.retain(|(tid, timer)| {
            if *tid == id {
                timer.stop();
                false
            } else {
                true
            }
        });
    });
    QUEUE.with(|q| q.borrow_mut().retain(|t| t.id != id));
    rebuild_model(popup_weak);
    update_input_region(popup_handler, popup_weak);
    if QUEUE.with(|q| q.borrow().is_empty()) {
        popup_handler.hide();
    }
}

fn push_toast(
    mut toast: crate::ToastData,
    popup_handler: &WinHandle,
    popup_weak: &slint::Weak<crate::NotificationPopup>,
) {
    let id = next_id();
    toast.id = id;
    let duration = if toast.duration_ms <= 0 {
        DEFAULT_DURATION_MS
    } else {
        toast.duration_ms
    };

    QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        queue.push(toast);
        if queue.len() > MAX_TOASTS {
            let removed = queue.remove(0);
            TIMERS.with(|t| {
                t.borrow_mut().retain(|(tid, timer)| {
                    if *tid == removed.id {
                        timer.stop();
                        false
                    } else {
                        true
                    }
                });
            });
        }
    });

    rebuild_model(popup_weak);
    update_input_region(popup_handler, popup_weak);
    popup_handler.show_again();

    let ph = popup_handler.clone();
    let pw = popup_weak.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::SingleShot,
        Duration::from_millis(duration as u64),
        move || {
            remove_toast(id, &ph, &pw);
        },
    );
    TIMERS.with(|t| t.borrow_mut().push((id, timer)));
}

pub fn push(title: &str, message: &str, icon: &str, severity: i32, duration_ms: i32) {
    let title = title.to_string();
    let message = message.to_string();
    let icon = icon.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        let (ph, pw) = match CTX.with(|c| c.borrow().clone()) {
            Some(ctx) => ctx,
            None => {
                log::warn!("[notification] push called before connect — ignoring");
                return;
            }
        };
        let toast = crate::ToastData {
            id: 0,
            title: title.into(),
            message: message.into(),
            icon: icon.into(),
            severity,
            duration_ms,
        };
        push_toast(toast, &ph, &pw);
    });
}

pub struct NotificationController;

impl NotificationController {
    pub fn connect(
        popup_handler: WinHandle,
        popup_weak: slint::Weak<crate::NotificationPopup>,
    ) {
        CTX.with(|c| {
            *c.borrow_mut() = Some((popup_handler.clone(), popup_weak.clone()));
        });

        if let Some(popup) = popup_weak.upgrade() {
            popup.global::<crate::NotificationAdapter>().on_dismiss(move |id| {
                let (ph, pw) = CTX.with(|c| c.borrow().clone().unwrap());
                remove_toast(id, &ph, &pw);
            });
        }
    }
}
