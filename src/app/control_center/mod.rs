use slint::ComponentHandle;
use slint_layer_shell::wayland_adapter::WinHandle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::thread::LocalKey;
use std::time::Duration;

// Dwell antes de abrir por hover: si el puntero solo pasa de camino a otro
// widget de la barra, el panel no llega a abrirse.
const HOVER_DWELL: Duration = Duration::from_millis(120);
// Gracia al salir: el panel arranca justo debajo de la barra, así que hay
// que dar tiempo a cruzar desde el widget hasta el panel. Corta a propósito:
// al dejar el hover la reacción tiene que sentirse inmediata.
const LEAVE_GRACE: Duration = Duration::from_millis(130);
// Mostrar el surface y recién después marcar is-open: la animación arranca
// desde arriba en vez de aparecer ya abierta.
const SHOW_DELAY: Duration = Duration::from_millis(16);
// Igual al `animate y` de salida del .slint (130ms): ocultar cuando
// terminó de subir.
const HIDE_DELAY: Duration = Duration::from_millis(140);

thread_local! {
    static HIDE_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
    static OPEN_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
    static HOVER_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
}

fn stop(timer: &'static LocalKey<RefCell<Option<slint::Timer>>>) {
    timer.with(|t| *t.borrow_mut() = None);
}

fn start(
    timer: &'static LocalKey<RefCell<Option<slint::Timer>>>,
    delay: Duration,
    callback: impl FnMut() + 'static,
) {
    timer.with(|t| {
        let new_timer = slint::Timer::default();
        new_timer.start(slint::TimerMode::SingleShot, delay, callback);
        *t.borrow_mut() = Some(new_timer);
    });
}

struct Panel {
    open: Cell<bool>,
    visible: Cell<bool>,
    handler: WinHandle,
    weak: slint::Weak<crate::ControlCenter>,
}

impl Panel {
    /// Los globals son por-window: `is-open` va a la instancia del panel,
    /// que es la que anima el contenido.
    fn set_open(&self, value: bool) {
        if let Some(window) = self.weak.upgrade() {
            window
                .global::<crate::ControlCenterAdapter>()
                .set_is_open(value);
        }
    }

    fn open(self: &Rc<Self>) {
        if self.open.replace(true) {
            return;
        }
        stop(&HIDE_TIMER);
        stop(&HOVER_TIMER);

        let me = self.clone();
        start(&OPEN_TIMER, SHOW_DELAY, move || {
            me.handler.show_again();
            me.visible.set(true);
            me.set_open(true);
        });
    }

    fn close(self: &Rc<Self>) {
        if !self.open.replace(false) {
            return;
        }
        stop(&HOVER_TIMER);
        stop(&OPEN_TIMER);
        self.set_open(false);

        // Si el show todavía no ocurrió, no hay surface que ocultar.
        if !self.visible.replace(false) {
            return;
        }
        let me = self.clone();
        start(&HIDE_TIMER, HIDE_DELAY, move || me.handler.hide());
    }

    fn toggle(self: &Rc<Self>) {
        if self.open.get() {
            self.close();
        } else {
            self.open();
        }
    }

    /// Hover sobre el widget de media: abre después del dwell.
    /// También cancela un cierre pendiente si el puntero vuelve.
    fn schedule_open(self: &Rc<Self>) {
        stop(&HOVER_TIMER);
        if self.open.get() {
            return;
        }
        let me = self.clone();
        start(&HOVER_TIMER, HOVER_DWELL, move || me.open());
    }

    /// Salida del widget o del panel: cierra con gracia.
    fn schedule_close(self: &Rc<Self>) {
        stop(&HOVER_TIMER);
        if !self.open.get() {
            return;
        }
        let me = self.clone();
        start(&HOVER_TIMER, LEAVE_GRACE, move || me.close());
    }

    fn keep_open(&self) {
        stop(&HOVER_TIMER);
    }
}

pub struct ControlCenterController;

impl ControlCenterController {
    pub fn connect(
        window: &crate::StatusBarWindow,
        ctrl_weak: slint::Weak<crate::ControlCenter>,
        ctrl_handler: WinHandle,
    ) {
        let adapter = window.global::<crate::ControlCenterAdapter>();
        let media = window.global::<crate::MediaAdapter>();

        let panel = Rc::new(Panel {
            open: Cell::new(false),
            visible: Cell::new(false),
            handler: ctrl_handler,
            weak: ctrl_weak.clone(),
        });

        // Click: sigue funcionando como toggle además del hover.
        let p = panel.clone();
        adapter.on_toggle_control_center(move || p.toggle());

        let p = panel.clone();
        media.on_hover_enter(move || p.schedule_open());

        let p = panel.clone();
        media.on_hover_leave(move || p.schedule_close());

        // Mientras el puntero esté dentro del panel no se cierra.
        let p = panel;
        if let Some(ctrl) = ctrl_weak.upgrade() {
            ctrl.global::<crate::ControlCenterAdapter>()
                .on_panel_hover(move |inside: bool| {
                    if inside {
                        p.keep_open();
                    } else {
                        p.schedule_close();
                    }
                });
        }
    }
}
