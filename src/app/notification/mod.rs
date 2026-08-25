use slint::{ComponentHandle, Model};
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

type Ctx = (
    WinHandle,
    slint::Weak<crate::NotificationPopup>,
    Rc<slint::VecModel<crate::ToastData>>,
);

thread_local! {
    static NEXT_ID: Cell<i32> = const { Cell::new(0) };
    static TIMERS: RefCell<Vec<(i32, slint::Timer)>> = const { RefCell::new(Vec::new()) };
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

fn next_id() -> i32 {
    NEXT_ID.with(|c| {
        let id = c.get() + 1;
        c.set(id);
        id
    })
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

fn forget_timer(id: i32) {
    TIMERS.with(|t| {
        t.borrow_mut().retain(|(tid, timer)| {
            if *tid == id {
                timer.stop();
                false
            } else {
                true
            }
        });
    });
}

fn remove_toast(id: i32, ctx: &Ctx) {
    let (popup_handler, popup_weak, model) = ctx;
    forget_timer(id);
    let row = (0..model.row_count()).find(|&i| model.row_data(i).is_some_and(|t| t.id == id));
    if let Some(idx) = row {
        model.remove(idx);
    }
    update_input_region(popup_handler, popup_weak);
    if model.row_count() == 0 {
        popup_handler.hide();
    }
}

fn push_toast(mut toast: crate::ToastData, ctx: &Ctx) {
    let (popup_handler, popup_weak, model) = ctx;
    let id = next_id();
    toast.id = id;
    let duration = if toast.duration_ms <= 0 {
        DEFAULT_DURATION_MS
    } else {
        toast.duration_ms
    };

    model.push(toast);
    if model.row_count() > MAX_TOASTS {
        let evicted = model.remove(0);
        forget_timer(evicted.id);
    }

    update_input_region(popup_handler, popup_weak);
    popup_handler.show_again();

    let timer = slint::Timer::default();
    let timer_ctx = (popup_handler.clone(), popup_weak.clone(), Rc::clone(model));
    timer.start(
        slint::TimerMode::SingleShot,
        Duration::from_millis(duration as u64),
        move || {
            remove_toast(id, &timer_ctx);
        },
    );
    TIMERS.with(|t| t.borrow_mut().push((id, timer)));
}

pub fn push(
    title: &str,
    message: &str,
    icon: &str,
    severity: i32,
    duration_ms: i32,
    tag: &str,
) {
    let title = title.to_string();
    let message = message.to_string();
    let icon = icon.to_string();
    let tag = tag.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        let Some(ctx) = CTX.with(|c| c.borrow().clone()) else {
            log::warn!("[notification] push called before connect — ignoring");
            return;
        };
        let toast = crate::ToastData {
            id: 0,
            title: title.into(),
            message: message.into(),
            icon: icon.into(),
            severity,
            duration_ms,
            tag: tag.into(),
        };
        push_toast(toast, &ctx);
    });
}

pub fn dismiss_tagged(tag: &str) {
    let tag = tag.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        let Some(ctx) = CTX.with(|c| c.borrow().clone()) else {
            return;
        };
        let (ref popup_handler, ref popup_weak, ref model) = ctx;
        let mut i = 0;
        while i < model.row_count() {
            if model.row_data(i).is_some_and(|t| t.tag.as_str() == tag.as_str()) {
                let removed = model.remove(i);
                forget_timer(removed.id);
            } else {
                i += 1;
            }
        }
        update_input_region(popup_handler, popup_weak);
        if model.row_count() == 0 {
            popup_handler.hide();
        }
    });
}

pub struct NotificationController;

impl NotificationController {
    pub fn connect(popup_handler: WinHandle, popup_weak: slint::Weak<crate::NotificationPopup>) {
        let model = Rc::new(slint::VecModel::from(Vec::<crate::ToastData>::new()));
        if let Some(popup) = popup_weak.upgrade() {
            popup.global::<crate::NotificationAdapter>().set_toasts(model.clone().into());
        }

        CTX.with(|c| *c.borrow_mut() = Some((popup_handler.clone(), popup_weak.clone(), model)));

        if let Some(popup) = popup_weak.upgrade() {
            popup.global::<crate::NotificationAdapter>().on_dismiss(move |id| {
                if let Some(ctx) = CTX.with(|c| c.borrow().clone()) {
                    remove_toast(id, &ctx);
                }
            });
        }
    }
}
