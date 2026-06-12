use hyprland::data::{Workspace, Workspaces};
use hyprland::event_listener::EventListener;
use hyprland::prelude::{HyprData, HyprDataActive};
use hyprland::shared::WorkspaceType;
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

struct SendEventListener(EventListener);
unsafe impl Send for SendEventListener {}

fn fetch_initial_state(total: i32, fmt: &Option<Vec<String>>) -> (Vec<i32>, i32, Vec<String>) {
    let size = total as usize + 1;
    let mut counts = vec![0i32; size];
    let labels: Vec<String>;

    if let Ok(workspaces) = Workspaces::get() {
        for ws in workspaces.iter() {
            let id = ws.id as usize;
            if id < size {
                counts[id] = ws.windows as i32;
            }
        }
        labels = if let Some(labels) = fmt {
            labels.clone()
        } else {
            (1..=total)
                .map(|id| {
                    workspaces
                        .iter()
                        .find(|ws| ws.id == id)
                        .map(|ws| ws.name.clone())
                        .unwrap_or_else(|| id.to_string())
                })
                .collect()
        };
    } else {
        labels = if let Some(labels) = fmt {
            labels.clone()
        } else {
            (1..=total).map(|i| i.to_string()).collect()
        };
    }

    let active = Workspace::get_active().ok().map(|a| a.id).unwrap_or(1);

    (counts, active, labels)
}

fn apply_occupied(adapter: &crate::WorkspacesAdapter, counts: &[i32]) {
    let occupied: Vec<bool> = counts.iter().skip(1).map(|&c| c > 0).collect();
    adapter.set_workspace_occupied(std::rc::Rc::new(slint::VecModel::from(occupied)).into());
}

fn try_schedule_update(
    w: &slint::Weak<crate::StatusBarWindow>,
    pending: &Arc<AtomicBool>,
    active: &Arc<AtomicI32>,
    counts: &Arc<Mutex<Vec<i32>>>,
) {
    if !pending.swap(true, Ordering::AcqRel) {
        let w = w.clone();
        let p = pending.clone();
        let a = active.clone();
        let c = counts.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let id = a.load(Ordering::Relaxed);
            if let Some(window) = w.upgrade() {
                let adapter = window.global::<crate::WorkspacesAdapter>();
                adapter.set_active_workspace(id);
                let counts = c.lock().unwrap_or_else(|e| e.into_inner());
                apply_occupied(&adapter, &counts);
            }
            p.store(false, Ordering::Release);
        });
    }
}

// fn log_event(event_type: &str, data: &dyn std::fmt::Debug) {
//     use std::io::Write;
//     if let Ok(mut f) = std::fs::OpenOptions::new()
//         .create(true)
//         .append(true)
//         .open("/tmp/hypr_events.log")
//     {
//         let _ = writeln!(
//             f,
//             "[{}] {event_type}\n{:#?}\n",
//             chrono::Local::now().format("%H:%M:%S%.3f"),
//             data
//         );
//     }
// }

pub struct WorkspacesController;

impl WorkspacesController {
    pub fn connect(window: &crate::StatusBarWindow) {
        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
            log::warn!("[workspaces] HYPRLAND_INSTANCE_SIGNATURE not set — Hyprland IPC disabled");
            return;
        }

        let cfg = crate::config::load_or_create_config();
        let total = cfg.workspaces.total_workspaces;
        let fmt = cfg.workspaces.format;

        let adapter = window.global::<crate::WorkspacesAdapter>();
        adapter.set_total_workspaces(total);

        let (counts, active, labels) = fetch_initial_state(total, &fmt);
        adapter.set_active_workspace(active);
        apply_occupied(&adapter, &counts);
        let labels: Vec<slint::SharedString> =
            labels.into_iter().map(|s| s.as_str().into()).collect();
        adapter.set_workspace_format(std::rc::Rc::new(slint::VecModel::from(labels)).into());

        let counts = Arc::new(Mutex::new(counts));
        let active = Arc::new(AtomicI32::new(active));
        let pending = Arc::new(AtomicBool::new(false));
        let weak = window.as_weak();

        std::thread::spawn(move || {
            let mut listener = SendEventListener(EventListener::new());

            let w = weak.clone();
            let a = active.clone();
            let c = counts.clone();
            let p = pending.clone();
            listener.0.add_workspace_changed_handler(move |data| {
                if !matches!(data.name, WorkspaceType::Regular(_)) {
                    return;
                }
                log::info!("[workspaces] workspace changed: id={}", data.id);
                a.store(data.id, Ordering::Relaxed);
                try_schedule_update(&w, &p, &a, &c);
            });

            let w = weak.clone();
            let a = active.clone();
            let c = counts.clone();
            let p = pending.clone();
            listener.0.add_window_opened_handler(move |data| {
                log::info!(
                    "[workspaces] window opened on workspace {}",
                    data.workspace_name
                );
                if let Ok(id) = data.workspace_name.parse::<i32>() {
                    let mut counts = c.lock().unwrap();
                    let idx = id as usize;
                    if idx < counts.len() {
                        counts[idx] += 1;
                    }
                }
                try_schedule_update(&w, &p, &a, &c);
            });

            let w = weak.clone();
            let a = active.clone();
            let c = counts.clone();
            let p = pending.clone();
            listener.0.add_window_moved_handler(move |data| {
                log::info!(
                    "[workspaces] window moved to workspace {}",
                    data.workspace_id
                );
                {
                    let mut counts = c.lock().unwrap();
                    let id = data.workspace_id as usize;
                    if id < counts.len() {
                        counts[id] += 1;
                    }
                }
                try_schedule_update(&w, &p, &a, &c);
            });

            let w = weak.clone();
            let a = active.clone();
            let c = counts.clone();
            let p = pending.clone();
            listener.0.add_workspace_deleted_handler(move |data| {
                log::info!("[workspaces] workspace deleted: id={}", data.id);
                {
                    let mut counts = c.lock().unwrap();
                    let id = data.id as usize;
                    if id < counts.len() {
                        counts[id] = 0;
                    }
                }
                try_schedule_update(&w, &p, &a, &c);
            });

            if let Err(e) = listener.0.start_listener() {
                log::error!("[workspaces] hyprland listener error: {e}");
            }
        });
    }
}
