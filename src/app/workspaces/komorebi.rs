use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use serde::Deserialize;
use uds_windows::{UnixListener, UnixStream};
use which::which;

const SOCKET_NAME: &str = "barritaEvents";

static LAST_WORKSPACE_STATE: Mutex<Option<WorkspaceInfo>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceInfo {
    pub active_workspace: i32,
    pub workspace_occupied: Vec<bool>,
}

#[derive(Debug, Deserialize)]
struct KomorebiEvent {
    event: EventInfo,
    #[serde(default)]
    state: Option<State>,
}

#[derive(Debug, Deserialize)]
struct EventInfo {
    #[serde(rename = "type")]
    event_type: String,
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

#[derive(Debug, Deserialize, Default)]
struct Workspace {
    #[serde(default)]
    containers: Containers,
}

#[derive(Debug, Deserialize, Default)]
struct Containers {
    #[serde(default)]
    elements: Vec<Container>,
}

#[derive(Debug, Deserialize, Default)]
struct Container {
    #[serde(default)]
    _windows: (),
}

pub fn start_komorebi_listener<F>(callback: F)
where
    F: Fn(WorkspaceInfo) + Send + 'static,
{
    thread::spawn(move || {
        if let Err(e) = run_listener(callback) {
            eprintln!("[komorebi] ERROR: {}", e);
        }
    });
}

fn run_listener<F>(callback: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn(WorkspaceInfo) + Send + 'static,
{
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
                if let Err(e) = read_events(stream, &callback) {
                    eprintln!("[komorebi] Read error: {}", e);
                }
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

fn read_events<F>(
    stream: UnixStream,
    callback: &F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn(WorkspaceInfo) + Send + 'static,
{
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(event) => {
                if let Some(info) = parse_workspace_event(&event) {
                    let should_update = {
                        let mut last_state = LAST_WORKSPACE_STATE.lock().unwrap();
                        let changed = last_state.as_ref() != Some(&info);
                        if changed {
                            *last_state = Some(info.clone());
                            true
                        } else {
                            false
                        }
                    };

                    if should_update {
                        // println!("[komorebi] WORKSPACE: active={}, occupied={:?}", 
                        //     info.active_workspace, 
                        //     info.workspace_occupied
                        // );

                        callback(info);
                    }
                }
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }
    Ok(())
}

fn parse_workspace_event(event: &str) -> Option<WorkspaceInfo> {
    let parsed: KomorebiEvent = serde_json::from_str(event).ok()?;

    let event_type = parsed.event.event_type.clone();

    if event_type != "FocusChange" && event_type != "WorkAreaChanged" {
        return None;
    }

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
    which("komorebic.exe").ok()
}