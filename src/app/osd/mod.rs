// OSD de volumen/brillo/mic. Event-driven, cero polling: `pactl subscribe`
// para audio, `inotify` sobre sysfs para brillo. Diff en hilo + un solo
// `invoke_from_event_loop` si cambió (patrón battery/media).

use inotify::{Inotify, WatchMask};
use slint::{ComponentHandle, Timer, TimerMode, Weak};
use slint_layer_shell::wayland_adapter::WinHandle;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

// Ventana 190x72 (ver osd.slint). Click-through: input vacía siempre.
const WINDOW_W: i32 = 190;
const WINDOW_H: i32 = 72;

// Espera al fade-out (180ms) antes de ocultar la surface.
const FADE_OUT_MS: u64 = 220;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OsdKind {
    Volume,
    Brightness,
    Mic,
}

#[derive(Clone, PartialEq)]
struct OsdState {
    kind: OsdKind,
    value: i32,
    muted: bool,
}

thread_local! {
    static HANDLER: RefCell<Option<WinHandle>> = const { RefCell::new(None) };
    static WEAK: RefCell<Option<Weak<crate::OsdPopup>>> = const { RefCell::new(None) };
    static TIMEOUT_MS: RefCell<u64> = const { RefCell::new(1400) };
    static LAST: RefCell<Option<OsdState>> = const { RefCell::new(None) };
    static HIDE_TIMER: RefCell<Timer> = RefCell::new(Timer::default());
    static FADE_TIMER: RefCell<Timer> = RefCell::new(Timer::default());
    static REVEAL_TIMER: RefCell<Timer> = RefCell::new(Timer::default());
    static PENDING: RefCell<Option<OsdState>> = const { RefCell::new(None) };
}

// La surface nueva pinta su primer frame con el valor final: con la
// píldora oculta, el valor se aplica diferido para que anime visible.
const REVEAL_DELAY_MS: u64 = 50;

fn apply_pending(weak: &Weak<crate::OsdPopup>) {
    let pending = PENDING.with(|p| p.borrow_mut().take());
    if let (Some(state), Some(popup)) = (pending, weak.upgrade()) {
        apply_state(&popup, state.kind, state.value, state.muted);
    }
}

fn icon_label_for(kind: OsdKind, value: i32, muted: bool) -> (&'static str, &'static str) {
    match kind {
        OsdKind::Volume => {
            let icon = if muted {
                "volume_off"
            } else if value == 0 {
                "volume_mute"
            } else if value < 50 {
                "volume_down"
            } else {
                "volume_up"
            };
            (icon, "Volumen")
        }
        OsdKind::Brightness => {
            let icon = if value < 34 {
                "brightness_low"
            } else if value < 67 {
                "brightness_medium"
            } else {
                "brightness_high"
            };
            (icon, "Brillo")
        }
        OsdKind::Mic => {
            if muted {
                ("mic_off", "Micrófono")
            } else {
                ("mic", "Micrófono")
            }
        }
    }
}

fn apply_state(popup: &crate::OsdPopup, kind: OsdKind, value: i32, muted: bool) {
    let adapter = popup.global::<crate::OsdAdapter>();
    let (icon, label) = icon_label_for(kind, value, muted);
    let percent = match kind {
        OsdKind::Mic => {
            if muted {
                "Off".into()
            } else {
                "On".into()
            }
        }
        _ => format!("{value}%").into(),
    };
    let kind_str: &str = match kind {
        OsdKind::Volume => "volume",
        OsdKind::Brightness => "brightness",
        OsdKind::Mic => "mic",
    };
    adapter.set_kind(kind_str.into());
    adapter.set_label(label.into());
    adapter.set_icon(icon.into());
    adapter.set_value(value);
    adapter.set_muted(muted);
    adapter.set_percent_text(percent);
}

fn hide_now(handler: &WinHandle, weak: &Weak<crate::OsdPopup>) {
    if let Some(popup) = weak.upgrade() {
        popup.global::<crate::OsdAdapter>().set_visible(false);
    }
    let h2 = handler.clone();
    let w2 = weak.clone();
    FADE_TIMER.with(|t| {
        t.borrow().start(
            TimerMode::SingleShot,
            Duration::from_millis(FADE_OUT_MS),
            move || {
                h2.hide();
                let _ = w2;
            },
        );
    });
}

// Overdrive PipeWire/Pulse: 150%. Brillo (ratio sysfs): 100.
const MAX_VOLUME: i32 = 150;
const MAX_BRIGHTNESS: i32 = 100;

fn show(kind: OsdKind, value: i32, muted: bool) {
    let max = match kind {
        OsdKind::Brightness => MAX_BRIGHTNESS,
        OsdKind::Volume | OsdKind::Mic => MAX_VOLUME,
    };
    let value = value.clamp(0, max);
    let _ = slint::invoke_from_event_loop(move || {
        let changed = LAST.with(|l| {
            let next = OsdState { kind, value, muted };
            let diff = l.borrow().as_ref() != Some(&next);
            *l.borrow_mut() = Some(next);
            diff
        });
        let (handler, weak, timeout) = HANDLER.with(|h| {
            WEAK.with(|w| {
                TIMEOUT_MS.with(|t| (h.borrow().clone(), w.borrow().clone(), *t.borrow()))
            })
        });
        let (Some(handler), Some(weak)) = (handler, weak) else {
            return;
        };
        let Some(popup) = weak.upgrade() else {
            return;
        };
        let adapter = popup.global::<crate::OsdAdapter>();
        let was_visible = adapter.get_visible();
        if changed {
            if was_visible {
                // Visible: anima en vivo, cancela reveals pendientes.
                REVEAL_TIMER.with(|t| t.borrow().stop());
                PENDING.with(|p| *p.borrow_mut() = None);
                apply_state(&popup, kind, value, muted);
            } else {
                // Oculta: muestra con el valor viejo, revela el nuevo al pintar.
                PENDING.with(|p| *p.borrow_mut() = Some(OsdState { kind, value, muted }));
                let w2 = weak.clone();
                REVEAL_TIMER.with(|t| {
                    t.borrow().start(
                        TimerMode::SingleShot,
                        Duration::from_millis(REVEAL_DELAY_MS),
                        move || apply_pending(&w2),
                    );
                });
            }
        }
        // Actividad en fade-out: cancela el hide pendiente.
        FADE_TIMER.with(|t| t.borrow().stop());
        if !was_visible {
            adapter.set_visible(true);
            handler.show_again();
        }
        HIDE_TIMER.with(|t| {
            let h2 = handler.clone();
            let w2 = weak.clone();
            t.borrow().start(
                TimerMode::SingleShot,
                Duration::from_millis(timeout),
                move || hide_now(&h2, &w2),
            );
        });
    });
}

// --- Audio (pactl + fallback wpctl) ---

// "Volume: front-left: 48497 /  74% / ..." -> 74
fn parse_pactl_pct(out: &str) -> Option<i32> {
    let i = out.find('%')?;
    out[..i]
        .rsplit(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn pactl_muted(out: &str) -> bool {
    out.rsplit(':')
        .next()
        .is_some_and(|s| s.trim().eq_ignore_ascii_case("yes"))
}

// "Volume: 0.74 [MUTED]" -> (74, muted). Acepta >1.0.
fn parse_wpctl(out: &str) -> Option<(i32, bool)> {
    let vol: f32 = out.split_whitespace().nth(1)?.parse().ok()?;
    Some((((vol * 100.0).round() as i32).clamp(0, MAX_VOLUME), out.contains("MUTED")))
}

fn pactl_level(volume_args: &[&str], mute_args: &[&str]) -> Option<(i32, bool)> {
    let out = Command::new("pactl")
        .args(volume_args)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let pct = parse_pactl_pct(&String::from_utf8_lossy(&out.stdout))?;
    let muted = Command::new("pactl")
        .args(mute_args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| pactl_muted(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(false);
    Some((pct, muted))
}

fn wpctl_level(device: &str) -> Option<(i32, bool)> {
    let out = Command::new("wpctl")
        .args(["get-volume", device])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    parse_wpctl(&String::from_utf8_lossy(&out.stdout))
}

fn query_sink() -> Option<(i32, bool)> {
    pactl_level(
        &["get-sink-volume", "@DEFAULT_SINK@"],
        &["get-sink-mute", "@DEFAULT_SINK@"],
    )
    .or_else(|| wpctl_level("@DEFAULT_AUDIO_SINK@"))
}

fn query_source() -> Option<(i32, bool)> {
    pactl_level(
        &["get-source-volume", "@DEFAULT_SOURCE@"],
        &["get-source-mute", "@DEFAULT_SOURCE@"],
    )
    .or_else(|| wpctl_level("@DEFAULT_AUDIO_SOURCE@"))
}

fn maybe_show(tag: &str, kind: OsdKind, cur: Option<(i32, bool)>, last: &mut Option<(i32, bool)>) {
    if let Some(cur) = cur.filter(|c| last.is_none_or(|l| l != *c)) {
        *last = Some(cur);
        log::info!("[osd] {tag} {}% muted={}", cur.0, cur.1);
        show(kind, cur.0, cur.1);
    }
}

// Mic binario: ignora el gain; 100 = abierto, 0 = muteado.
fn mic_state() -> Option<(i32, bool)> {
    query_source().map(|(_, muted)| if muted { (0, true) } else { (100, false) })
}

fn spawn_audio_listener() {
    std::thread::spawn(|| {
        // Prime: sin píldora al arrancar.
        let mut last_sink = query_sink();
        let mut last_source = mic_state();

        let mut child = match Command::new("pactl")
            .arg("subscribe")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[osd] pactl subscribe failed: {e} — audio OSD disabled");
                return;
            }
        };
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                log::warn!("[osd] pactl subscribe has no stdout — audio OSD disabled");
                return;
            }
        };
        log::info!("[osd] listening for PulseAudio/PipeWire events");

        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.contains("on sink") || line.contains("on server") {
                maybe_show("volume", OsdKind::Volume, query_sink(), &mut last_sink);
            }
            if line.contains("on source") || line.contains("on server") {
                maybe_show("mic", OsdKind::Mic, mic_state(), &mut last_source);
            }
        }
        log::warn!("[osd] pactl subscribe ended — audio OSD disabled");
    });
}

// --- Brillo (inotify sobre sysfs) ---

fn find_backlight() -> Option<(PathBuf, i64)> {
    std::fs::read_dir("/sys/class/backlight")
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter_map(|p| {
            let max: i64 = std::fs::read_to_string(p.join("max_brightness"))
                .ok()?
                .trim()
                .parse()
                .ok()?;
            (max > 0).then_some((p, max))
        })
        .min_by(|a, b| a.0.cmp(&b.0))
}

fn read_brightness_pct(dir: &std::path::Path, max: i64) -> Option<i32> {
    let cur: i64 = std::fs::read_to_string(dir.join("brightness"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some((((cur * 100) / max.max(1)) as i32).clamp(0, 100))
}

fn spawn_backlight_listener() {
    std::thread::spawn(|| {
        let Some((dir, max)) = find_backlight() else {
            log::warn!("[osd] no backlight device — brightness OSD disabled");
            return;
        };
        log::info!("[osd] watching brightness at {}", dir.display());

        let mut inotify = match Inotify::init() {
            Ok(i) => i,
            Err(e) => {
                log::warn!("[osd] inotify init failed: {e} — brightness OSD disabled");
                return;
            }
        };
        if let Err(e) = inotify.watches().add(&dir, WatchMask::MODIFY) {
            log::warn!("[osd] inotify watch failed: {e} — brightness OSD disabled");
            return;
        }

        let mut last: Option<(i32, bool)> =
            read_brightness_pct(&dir, max).map(|p| (p, false));
        let mut buf = [0u8; 1024];
        loop {
            let mut events = match inotify.read_events_blocking(&mut buf) {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("[osd] inotify read failed: {e} — brightness OSD disabled");
                    return;
                }
            };
            let touched = events.any(|e| {
                e.name
                    .as_ref()
                    .is_some_and(|n| n.to_string_lossy() == "brightness")
            });
            if !touched {
                continue;
            }
            let cur = read_brightness_pct(&dir, max).map(|p| (p, false));
            maybe_show("brightness", OsdKind::Brightness, cur, &mut last);
        }
    });
}

pub struct OsdController;

impl OsdController {
    pub fn connect(osd_handler: WinHandle, osd_weak: Weak<crate::OsdPopup>) {
        // Click-through permanente.
        osd_handler.subtract_input_region(0, 0, WINDOW_W, WINDOW_H);

        let cfg = crate::config::load_or_create_config().osd;
        HANDLER.with(|h| *h.borrow_mut() = Some(osd_handler));
        WEAK.with(|w| *w.borrow_mut() = Some(osd_weak));
        TIMEOUT_MS.with(|t| *t.borrow_mut() = cfg.timeout_ms);

        spawn_audio_listener();
        spawn_backlight_listener();
    }
}
