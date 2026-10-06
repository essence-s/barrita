use slint::{ComponentHandle, Timer, TimerMode};
use slint_layer_shell::wayland_adapter::WinHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

// Auto-hide event-driven: la franja "dock-hotspot" despierta el dock y
// el hover de la barra lo mantiene visible. Sin hilos ni polling:
// en reposo nada se ejecuta hasta que el compositor empuja un enter.
const SHOW_DWELL: Duration = Duration::from_millis(50);

struct Autohide {
    visible: bool,
    hide_timeout: Duration,
    dock: WinHandle,
    show_timer: Timer,
    hide_timer: Timer,
}

impl Autohide {
    fn show_now(&mut self) {
        if !self.visible {
            self.visible = true;
            self.dock.show_again();
        }
    }

    fn hide_now(&mut self) {
        if self.visible {
            self.visible = false;
            self.dock.hide();
        }
    }

    fn schedule_hide(state: &Rc<RefCell<Self>>) {
        let (visible, timeout) = {
            let st = state.borrow();
            (st.visible, st.hide_timeout)
        };
        if visible {
            let s2 = state.clone();
            state.borrow_mut().hide_timer.start(TimerMode::SingleShot, timeout, move || {
                s2.borrow_mut().hide_now();
            });
        }
    }
}

pub struct DockController;

impl DockController {
    pub fn connect(
        dock: &crate::Dock,
        hotspot: &crate::DockHotspot,
        dock_handler: WinHandle,
        hotspot_handler: WinHandle,
    ) {
        let cfg = crate::config::load_or_create_config().dock;
        if !cfg.autohide {
            hotspot_handler.hide();
            log::info!("[dock] autohide disabled");
            return;
        }

        let state = Rc::new(RefCell::new(Autohide {
            visible: true,
            hide_timeout: Duration::from_millis(cfg.hide_timeout_ms),
            dock: dock_handler,
            show_timer: Timer::default(),
            hide_timer: Timer::default(),
        }));

        // Hide inicial: nace visible, se oculta solo si nadie lo reclama
        // (un entered lo cancela).
        Autohide::schedule_hide(&state);

        for adapter in [
            dock.global::<crate::DockAdapter>(),
            hotspot.global::<crate::DockAdapter>(),
        ] {
            let s = state.clone();
            adapter.on_hotspot_entered(move || {
                let st = s.borrow_mut();
                st.hide_timer.stop();
                if !st.visible {
                    let s2 = s.clone();
                    st.show_timer.start(TimerMode::SingleShot, SHOW_DWELL, move || {
                        s2.borrow_mut().show_now();
                    });
                }
            });

            let s = state.clone();
            adapter.on_hotspot_left(move || {
                s.borrow_mut().show_timer.stop();
                Autohide::schedule_hide(&s);
            });

            let s = state.clone();
            adapter.on_dock_entered(move || {
                s.borrow_mut().hide_timer.stop();
            });

            let s = state.clone();
            adapter.on_dock_left(move || {
                Autohide::schedule_hide(&s);
            });

            adapter.on_tile_clicked(move |idx| {
                if idx >= 0 {
                    log::info!("[dock] tile clicked: {idx}");
                }
            });
        }
    }
}
