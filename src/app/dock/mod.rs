use freedesktop_desktop_entry::DesktopEntry;
use image::GenericImage;
use hyprland::data::{Client, Clients};
use hyprland::event_listener::EventListener;
use hyprland::prelude::*;
use hyprland::shared::HyprDataVec;
use slint::{ComponentHandle, Model, SharedString, Timer, TimerMode};
use slint_layer_shell::wayland_adapter::WinHandle;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Auto-hide event-driven: la franja "dock-hotspot" despierta el dock y
// el hover de la barra lo mantiene visible. Sin hilos de polling:
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
const ICON_THUMB: u32 = 96;

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

fn tile_size_for(count: i32) -> f32 {
    let n = count.max(1) as f32;
    TILE_MAX.min(TILE_MIN.max((BAR_CAP - TILE_PAD_X - (n - 1.0) * TILE_GAP) / n))
}

// Clicks solo en la barra: resetea la region y suma el rect util con
// 2px de margen hacia dentro.
fn apply_input_for_count(handler: &WinHandle, count: i32) {
    if count <= 0 {
        handler.subtract_input_region(0, 0, WIN_W, WIN_H);
        return;
    }
    let t = tile_size_for(count);
    let n = count as f32;
    let w = n * t + (n - 1.0) * TILE_GAP + TILE_PAD_X;
    let h = t + BAR_PAD_TOP + BAR_PAD_BOTTOM;
    let x0 = ((WIN_W as f32 - w) / 2.0 + 2.0).round() as i32;
    let y0 = ((WIN_H as f32 - LAYOUT_PAD_BOTTOM - h) + 2.0).round() as i32;
    handler.subtract_input_region(0, 0, WIN_W, WIN_H);
    handler.add_input_region(x0, y0, (w - 4.0).round() as i32, (h - 4.0).round() as i32);
}

struct SendEventListener(EventListener);
unsafe impl Send for SendEventListener {}

#[derive(Clone, Default)]
struct RawIcon {
    pixels: Vec<u8>,
    w: u32,
    h: u32,
}

struct DesktopInfo {
    id: String,
    wm_class: Option<String>,
    icon: Option<String>,
    exec_first: Option<String>,
}

#[derive(Clone)]
struct RawTile {
    address: String,
    app_id: String,
    title: String,
    icon: Option<RawIcon>,
    active: bool,
}

fn exec_first_token(exec: &str) -> Option<String> {
    exec.split_whitespace()
        .find(|t| !t.starts_with('%'))
        .and_then(|t| t.rsplit('/').next())
        .map(|s| s.to_lowercase())
}

fn scan_desktops() -> Vec<DesktopInfo> {
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    dirs.push(PathBuf::from("/usr/local/share/applications"));
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));

    let locales: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "desktop") {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_lowercase();
            let Ok(de) = DesktopEntry::from_path(&path, Some(&locales)) else {
                continue;
            };
            out.push(DesktopInfo {
                id,
                wm_class: de.startup_wm_class().map(|s| s.to_lowercase()),
                icon: de.icon().map(|s| s.to_string()),
                exec_first: de.exec().and_then(exec_first_token),
            });
        }
    }
    log::info!("[dock] desktop entries: {}", out.len());
    out
}

fn match_desktop<'a>(desktops: &'a [DesktopInfo], candidates: &[String]) -> Option<&'a DesktopInfo> {
    for pass in 0..3 {
        for c in candidates {
            let cl = c.to_lowercase();
            if cl.is_empty() {
                continue;
            }
            let hit = desktops.iter().find(|d| match pass {
                0 => d.wm_class.as_deref() == Some(cl.as_str()),
                1 => d.id == cl,
                _ => d.exec_first.as_deref() == Some(cl.as_str()),
            });
            if hit.is_some() {
                return hit;
            }
        }
    }
    None
}

fn normalize_rgba(img: image::RgbaImage) -> Option<RawIcon> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    // bbox del contenido (alfa): recorta y repone aire parejo.
    let (mut x0, mut y0) = (w, h);
    let (mut x1, mut y1) = (0u32, 0u32);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 8 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x1 < x0 || y1 < y0 {
        return None;
    }
    let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
    // Con fondo (cobertura ~total): a sangre, sin aire. Si no: 12%.
    let coverage = (cw * ch) as f32 / (w * h) as f32;
    let pad = if coverage >= 0.93 {
        0
    } else {
        ((cw.max(ch) as f32) * 0.12) as u32
    };
    let side = cw.max(ch) + pad * 2;
    let cropped = image::imageops::crop_imm(&img, x0, y0, cw, ch).to_image();
    let mut canvas = image::RgbaImage::new(side, side);
    canvas.copy_from(&cropped, (side - cw) / 2, (side - ch) / 2).ok()?;
    // CatmullRom en vez de Lanczos3: menos ringing en bordes con alfa.
    let out = image::imageops::resize(&canvas, ICON_THUMB, ICON_THUMB, image::imageops::FilterType::CatmullRom);
    let (w, h) = (out.width(), out.height());
    Some(RawIcon {
        pixels: out.into_raw(),
        w,
        h,
    })
}

fn load_icon_file(path: &Path) -> Option<RawIcon> {
    let img = image::open(path).ok()?.to_rgba8();
    if img.width().max(img.height()) < 48 {
        return None; // muy chico: mejor letra que blur
    }
    normalize_rgba(img)
}

fn load_svg_file(path: &Path) -> Option<RawIcon> {
    let bytes = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return None;
    }
    // Render a 256 y normaliza igual que el PNG (bbox + aire).
    let s = 256.0 / size.width().max(size.height());
    let (ox, oy) = ((256.0 - size.width() * s) / 2.0, (256.0 - size.height() * s) / 2.0);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(256, 256)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_row(s, 0.0, 0.0, s, ox, oy),
        &mut pixmap.as_mut(),
    );
    let img = image::RgbaImage::from_raw(256, 256, unpremultiply(pixmap.data()))?;
    normalize_rgba(img)
}

// tiny-skia entrega premultiplicados, el pipeline usa straight.
fn unpremultiply(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for px in data.chunks_exact(4) {
        let a = px[3] as u32;
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            out.push(((px[0] as u32 * 255 / a).min(255)) as u8);
            out.push(((px[1] as u32 * 255 / a).min(255)) as u8);
            out.push(((px[2] as u32 * 255 / a).min(255)) as u8);
            out.push(px[3]);
        }
    }
    out
}

fn load_any(path: &Path) -> Option<RawIcon> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("svg") | Some("svgz") => load_svg_file(path),
        _ => load_icon_file(path),
    }
}

// Iconos fuera de los temas (Flatpak): mismo arbol hicolor bajo
// .../flatpak/exports/share/icons. PNG mayor gana, luego SVG.
fn flatpak_icon_path(name: &str) -> Option<PathBuf> {
    let mut bases = vec![PathBuf::from("/var/lib/flatpak/exports/share/icons")];
    if let Some(home) = dirs::home_dir() {
        bases.push(home.join(".local/share/flatpak/exports/share/icons"));
    }
    let mut best_png: Option<(u32, PathBuf)> = None;
    let mut svg: Option<PathBuf> = None;
    for base in bases {
        let Ok(rd) = std::fs::read_dir(base.join("hicolor")) else {
            continue;
        };
        for entry in rd.flatten() {
            let dir = entry.path();
            let Some(dir_name) = dir.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if dir_name == "scalable" {
                let p = dir.join("apps").join(format!("{name}.svg"));
                if svg.is_none() && p.exists() {
                    svg = Some(p);
                }
                continue;
            }
            let Some(size) = dir_name.split('x').next().and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let p = dir.join("apps").join(format!("{name}.png"));
            if p.exists() && size >= best_png.as_ref().map(|(s, _)| *s).unwrap_or(0) {
                best_png = Some((size, p));
            }
        }
    }
    best_png.map(|(_, p)| p).or(svg)
}

fn resolve_icon(icon: &str) -> Option<RawIcon> {
    if icon.starts_with('/') {
        return load_any(Path::new(icon));
    }
    // 256: match exacto PNG en firefox/kitty, 128 por cercania en zen
    // (PNG primero por defecto; el exact-size gana a scalable).
    if let Some(raw) = freedesktop_icons::lookup(icon)
        .with_size(256)
        .with_cache()
        .find()
        .and_then(|p| load_any(&p))
    {
        return Some(raw);
    }
    flatpak_icon_path(icon).and_then(|p| load_any(&p))
}

fn group_key(c: &Client) -> String {
    if !c.class.is_empty() {
        c.class.to_lowercase()
    } else if !c.title.is_empty() {
        format!("t:{}", c.title.to_lowercase())
    } else {
        c.address.to_string()
    }
}

fn build_tiles(
    clients: &[Client],
    active_addr: Option<&str>,
    desktops: &[DesktopInfo],
    icon_cache: &mut HashMap<String, Option<RawIcon>>,
    order: &mut Vec<String>,
) -> Vec<RawTile> {
    let mut groups: HashMap<String, Vec<Client>> = HashMap::new();
    for c in clients {
        groups.entry(group_key(c)).or_default().push(c.clone());
    }
    // Orden estable: se conserva la posicion, las nuevas van al final,
    // las cerradas salen. El focus solo mueve highlight/dot.
    order.retain(|k| groups.contains_key(k));
    let mut fresh: Vec<(String, i8)> = groups
        .keys()
        .filter(|k| !order.contains(k))
        .map(|k| {
            let m = groups[k].iter().map(|c| c.focus_history_id).min().unwrap_or(i8::MAX);
            (k.clone(), m)
        })
        .collect();
    fresh.sort_by_key(|(_, m)| *m);
    order.extend(fresh.into_iter().map(|(k, _)| k));

    order
        .iter()
        .filter_map(|key| {
            let mut g = groups.remove(key)?;
            g.sort_by_key(|c| c.focus_history_id);
            let rep = &g[0];
            let candidates = [rep.class.clone(), rep.initial_class.clone()];
            let desktop = match_desktop(desktops, &candidates);
            let app_id = desktop
                .map(|d| d.id.clone())
                .unwrap_or_else(|| format!("unknown:{}", group_key(rep)));
            let icon = icon_cache
                .entry(app_id.clone())
                .or_insert_with(|| desktop.and_then(|d| d.icon.as_deref()).and_then(resolve_icon))
                .clone();
            let title = if rep.title.is_empty() {
                rep.class.clone()
            } else {
                rep.title.clone()
            };
            Some(RawTile {
                address: rep.address.to_string(),
                app_id,
                title,
                icon,
                active: active_addr.is_some_and(|a| g.iter().any(|c| c.address.to_string() == a)),
            })
        })
        .collect()
}

fn letter_for(title: &str) -> SharedString {
    title
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
        .into()
}

fn update_ui(weak: &slint::Weak<crate::Dock>, tiles: Vec<RawTile>) {
    let w = weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(window) = w.upgrade() {
            let apps: Vec<crate::DockApp> = tiles
                .into_iter()
                .map(|t| {
                    let (icon, has_icon) = match t.icon {
                        Some(raw) => (
                            crate::ui::image::rgba_to_slint_image(raw.pixels, raw.w, raw.h),
                            true,
                        ),
                        None => (slint::Image::default(), false),
                    };
                    crate::DockApp {
                        address: t.address.into(),
                        app_id: t.app_id.into(),
                        title: t.title.clone().into(),
                        icon,
                        has_icon,
                        letter: letter_for(&t.title),
                        active: t.active,
                    }
                })
                .collect();
            let adapter = window.global::<crate::DockAdapter>();
            adapter.set_tile_count(apps.len() as i32);
            adapter.set_apps(Rc::new(slint::VecModel::from(apps)).into());
        }
    });
}

fn refresh(
    weak: &slint::Weak<crate::Dock>,
    desktops: &Arc<Vec<DesktopInfo>>,
    icon_cache: &Arc<Mutex<HashMap<String, Option<RawIcon>>>>,
    order: &Arc<Mutex<Vec<String>>>,
) {
    let clients = match Clients::get() {
        Ok(c) => c.to_vec(),
        Err(e) => {
            log::warn!("[dock] clients failed: {e}");
            return;
        }
    };
    let active = Client::get_active()
        .ok()
        .flatten()
        .map(|c| c.address.to_string());
    let tiles = {
        let mut cache = icon_cache.lock().unwrap();
        let mut order = order.lock().unwrap();
        build_tiles(&clients, active.as_deref(), desktops, &mut cache, &mut order)
    };
    update_ui(weak, tiles);
}

fn focus_address(address: String) {
    // Hyprland >=0.55 (era Lua): el dispatch por socket se evalua como
    // Lua, la forma textual clasica falla. Se usa el dispatcher Lua
    // verificado en sesion: hl.dsp.focus({ window = "address:0x..." }).
    // Command directo sin shell: sin problemas de comillas.
    std::thread::spawn(move || {
        let expr = format!("hl.dsp.focus({{ window = \"address:{address}\" }})");
        match std::process::Command::new("hyprctl").args(["dispatch", &expr]).output() {
            Ok(out)
                if out.status.success() && out.stdout.starts_with(b"ok") => {}
            Ok(out) => {
                log::error!(
                    "[dock] focus failed: status={} stdout={} stderr={}",
                    out.status,
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr),
                );
            }
            Err(e) => log::error!("[dock] hyprctl exec failed: {e}"),
        }
    });
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
        }

        // apps-changed -> input recortado a la barra (hilo Slint, el
        // WinHandle no cruza threads).
        let weak = dock.as_weak();
        let input_handler = state.borrow().dock.clone();
        dock.global::<crate::DockAdapter>().on_apps_changed(move || {
            if let Some(window) = weak.upgrade() {
                let n = window.global::<crate::DockAdapter>().get_apps().row_count() as i32;
                apply_input_for_count(&input_handler, n);
            }
        });

        // Click -> enfoca la ventana (direccion leida del modelo).
        let weak = dock.as_weak();
        dock.global::<crate::DockAdapter>().on_tile_clicked(move |v: f32| {
            let idx = v.floor() as usize;
            if let Some(window) = weak.upgrade() {
                if let Some(app) = window.global::<crate::DockAdapter>().get_apps().row_data(idx)
                {
                    focus_address(app.address.to_string());
                }
            }
        });

        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
            log::warn!("[dock] HYPRLAND_INSTANCE_SIGNATURE not set — taskbar disabled");
            return;
        }

        let weak = dock.as_weak();
        std::thread::spawn(move || {
            let desktops = Arc::new(scan_desktops());
            let icon_cache = Arc::new(Mutex::new(HashMap::new()));
            let order = Arc::new(Mutex::new(Vec::new()));

            refresh(&weak, &desktops, &icon_cache, &order);

            let mut listener = SendEventListener(EventListener::new());
            macro_rules! on_rebuild {
                ($add:ident) => {{
                    let w = weak.clone();
                    let d = desktops.clone();
                    let c = icon_cache.clone();
                    let o = order.clone();
                    listener.0.$add(move |_| refresh(&w, &d, &c, &o));
                }};
            }
            on_rebuild!(add_window_opened_handler);
            on_rebuild!(add_window_closed_handler);
            on_rebuild!(add_window_moved_handler);
            on_rebuild!(add_active_window_changed_handler);

            if let Err(e) = listener.0.start_listener() {
                log::error!("[dock] hyprland listener error: {e}");
            }
        });
    }
}
