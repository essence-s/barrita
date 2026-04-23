use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use uds_windows::{UnixListener, UnixStream};
use which::which;

const SOCKET_NAME: &str = "barritaEvents";

pub fn start_komorebi_listener() {
    thread::spawn(|| {
        if let Err(e) = run_listener() {
            eprintln!("[komorebi] ERROR: {}", e);
        }
    });
}

fn run_listener() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                println!("[komorebi] Client connected from: {:?}", _addr);
                if let Err(e) = read_events(stream) {
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

fn read_events(stream: UnixStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(event) => {
                println!("[komorebi] EVENT: {}", event);
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }
    Ok(())
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