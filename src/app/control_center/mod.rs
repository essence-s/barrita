use slint::ComponentHandle;
use slint_layer_shell::wayland_adapter::WinHandle;
use std::cell::RefCell;

thread_local! {
    static HIDE_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
    static OPEN_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
}

pub struct ControlCenterController;

impl ControlCenterController {
    pub fn connect(window: &crate::StatusBarWindow, ctrl_handler: WinHandle) {
        let adapter = window.global::<crate::ControlCenterAdapter>();

        let ctrl = ctrl_handler.clone();
        let weak = window.as_weak();
        adapter.on_toggle_control_center(move || {
            let win = weak.unwrap();
            let adapter = win.global::<crate::ControlCenterAdapter>();
            let is_open = adapter.get_is_open();

            if is_open {
                // Cerrar: cancelar timer de apertura pendiente
                OPEN_TIMER.with(|t| *t.borrow_mut() = None);

                adapter.set_is_open(false);
                let ctrl = ctrl.clone();
                HIDE_TIMER.with(|t| {
                    let mut timer = t.borrow_mut();
                    let new_timer = slint::Timer::default();
                    new_timer.start(
                        slint::TimerMode::SingleShot,
                        std::time::Duration::from_millis(210),
                        move || {
                            ctrl.hide();
                        },
                    );
                    *timer = Some(new_timer);
                });
            } else {
                // Abrir: cancelar timer de cierre pendiente
                HIDE_TIMER.with(|t| *t.borrow_mut() = None);

                // Posicionar contenido arriba del viewport (surface aún oculto)
                adapter.set_is_open(false);

                // Mostrar surface 16ms después, cuando Slint ya posicionó el contenido
                let ctrl2 = ctrl_handler.clone();
                let weak2 = weak.clone();
                OPEN_TIMER.with(|t| {
                    let mut timer = t.borrow_mut();
                    let new_timer = slint::Timer::default();
                    new_timer.start(
                        slint::TimerMode::SingleShot,
                        std::time::Duration::from_millis(16),
                        move || {
                            ctrl2.show_again();
                            let weak3 = weak2.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak3.upgrade() {
                                    w.global::<crate::ControlCenterAdapter>()
                                        .set_is_open(true);
                                }
                            });
                        },
                    );
                    *timer = Some(new_timer);
                });
            }
        });
    }
}
