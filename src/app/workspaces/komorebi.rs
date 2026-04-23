use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use serde::Deserialize;
use slint::Weak;
use uds_windows::{UnixListener, UnixStream};
use which::which;
use crate::StatusBarWindow;

const SOCKET_NAME: &str = "barritaEvents";

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub active_workspace: i32,
    pub workspace_occupied: Vec<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct KomorebiEvent {
    event: EventInfo,
    #[serde(default)]
    state: Option<State>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EventInfo {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    content: serde_json::Value,
}

#[derive(Debug, Deserialize, Default)]
struct State {
    #[serde(default)]
    monitors: Monitors,
}

#[derive(Debug, Deserialize, Default)]
struct Monitors {
    #[serde(default)]
    elements: Vec<Monitor>,
}

#[derive(Debug, Deserialize, Default)]
struct Monitor {
    #[serde(default)]
    workspaces: Option<Workspaces>,
}

#[derive(Debug, Deserialize, Default)]
struct Workspaces {
    #[serde(default)]
    elements: Vec<Workspace>,
    #[serde(default)]
    focused: i32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
struct Workspace {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    containers: Containers,
}

#[derive(Debug, Deserialize, Default)]
struct Containers {
    #[serde(default)]
    elements: Vec<Container>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
struct Container {
    #[serde(default)]
    windows: Windows,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
struct Windows {
    #[serde(default)]
    elements: Vec<serde_json::Value>,
}

pub fn start_komorebi_listener(app_weak: Weak<StatusBarWindow>) {
    thread::spawn(move || {
        if let Err(e) = run_listener(app_weak) {
            eprintln!("[komorebi] ERROR: {}", e);
        }
    });
}

fn run_listener(app_weak: Weak<StatusBarWindow>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket_path = get_socket_path();
    let socket_name = SOCKET_NAME;

    println!("[komorebi] Creating socket at: {}", socket_path.display());

    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path)?;
    listener.set_nonblocking(true)?;

    println!("[komorebi] Socket created, starting komorebic subscribe-socket...");

    let komorebic_path = which_komorebic();
    if komorebic_path.is_none() {
        eprintln!("[komorebi] ERROR: komorebic.exe not found in PATH");
        return Err("komorebic.exe not found".into());
    }

    let komorebic_path = komorebic_path.unwrap();
    let _child = Command::new(&komorebic_path)
        .args(["subscribe-socket", socket_name])
        .spawn()?;

    println!("[komorebi] spawned komorebic subscribe-socket");

    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                println!("[komorebi] Client connected");
                if let Err(e) = read_events(stream, &app_weak) {
                    eprintln!("[komorebi] Read error: {}", e);
                }
                println!("[komorebi] Client disconnected");
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("[komorebi] Accept error: {}", e);
                return Err(Box::new(e));
            }
        }
    }
}

fn read_events(
    stream: UnixStream,
    app_weak: &Weak<StatusBarWindow>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(event) => {
                if let Some(info) = parse_workspace_event(&event) {
                    println!("[komorebi] WORKSPACE: active={}, occupied={:?}", 
                        info.active_workspace, 
                        info.workspace_occupied
                    );

                    let weak = app_weak.clone();
                    let info_clone = info.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(app) = weak.upgrade() {
                            update_workspace_widget(&app, &info_clone);
                        }
                    })?;
                }
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }
    Ok(())
}

fn update_workspace_widget(window: &StatusBarWindow, info: &WorkspaceInfo) {
    let occupied: Vec<bool> = info.workspace_occupied.clone();
    let model = slint::VecModel::from(occupied);
    window.set_active_workspace(info.active_workspace);
    window.set_workspace_occupied(slint::ModelRc::new(model));
}

fn parse_workspace_event(event: &str) -> Option<WorkspaceInfo> {
    let parsed: KomorebiEvent = serde_json::from_str(event).ok()?;

    let state = parsed.state?;

    let monitor = state.monitors.elements.first()?;
    let workspaces = monitor.workspaces.as_ref()?;

    let active_workspace = workspaces.focused + 1;

    let workspace_occupied: Vec<bool> = workspaces.elements.iter().map(|w| {
        !w.containers.elements.is_empty()
    }).collect();

    Some(WorkspaceInfo {
        active_workspace,
        workspace_occupied,
    })
}

fn get_socket_path() -> PathBuf {
    let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
    PathBuf::from(localappdata).join("komorebi").join(SOCKET_NAME)
}

fn which_komorebic() -> Option<PathBuf> {
    let paths = ["komorebic.exe", "komorebi\\komorebic.exe"];

    for path in &paths {
        if let Ok(output) = Command::new("where").arg(path).output() {
            if output.status.success() {
                let first_line = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()?
                    .trim()
                    .to_string();
                return Some(PathBuf::from(first_line));
            }
        }
    }

    which("komorebic.exe").ok()
}