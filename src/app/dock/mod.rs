use slint::{ComponentHandle, Timer, TimerMode};
use slint_layer_shell::wayland_adapter::WinHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

// Auto-hide event-driven: la franja "dock-hotspot" despierta el dock y
// el hover de la barra lo mantiene visible. Sin hilos ni polling:
// en reposo nada se ejecuta hasta que el compositor empuja un enter.
const SHOW_DWELL: Duration = Duration::from_millis(50);

// Geometria compartida con dock.slint (ventana 1366x70, padding 7/7/7/8,
// spacing 4, tope de barra 1100). Si cambian, cambiar ambos lados.
const WIN_W: i32 = 1366;
const WIN_H: i32 = 70;
const BAR_CAP: f32 = 1100.0;
const TILE_MAX: f32 = 40.0;
const TILE_MIN: f32 = 24.0;
const TILE_PAD_X: f32 = 14.0;
const TILE_GAP: f32 = 4.0;
const BAR_PAD_TOP: f32 = 7.0;
const BAR_PAD_BOTTOM: f32 = 8.0;
const LAYOUT_PAD_BOTTOM: f32 = 9.0;

fn tile_size_for(count: i32) -> f32 {
    let n = count.max(1) as f32;
    TILE_MAX.min(TILE_MIN.max((BAR_CAP - TILE_PAD_X - (n - 1.0) * TILE_GAP) / n))
}

// Clicks solo en la barra: resetea la region y suma el rect util con
// 2px de margen hacia dentro.
fn apply_input_for_count(handler: &WinHandle, count: i32) {
    let t = tile_size_for(count);
    let n = count.max(1) as f32;
    let w = n * t + (n - 1.0) * TILE_GAP + TILE_PAD_X;
    let h = t + BAR_PAD_TOP + BAR_PAD_BOTTOM;
    let x0 = ((WIN_W as f32 - w) / 2.0 + 2.0).round() as i32;
    let y0 = ((WIN_H as f32 - LAYOUT_PAD_BOTTOM - h) + 2.0).round() as i32;
    handler.subtract_input_region(0, 0, WIN_W, WIN_H);
    handler.add_input_region(x0, y0, (w - 4.0).round() as i32, (h - 4.0).round() as i32);
}

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
        let count = dock.global::<crate::DockAdapter>().get_tile_count();
        apply_input_for_count(&dock_handler, count);
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

            adapter.on_tile_clicked(move |v: f32| {
                let idx = v.floor() as i32;
                if idx >= 0 && idx < count {
                    log::info!("[dock] tile clicked: {idx}");
                }
            });
        }
    }
}
