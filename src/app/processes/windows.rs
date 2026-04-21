use crate::core::data::ProcessInfo;
use sysinfo::{ProcessesToUpdate, System};

pub fn get_top_process() -> ProcessInfo {
    let mut sys = System::new();

    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut processes: Vec<_> = sys
        .processes()
        .iter()
        .map(|(_pid, process)| {
            (
                process.name().to_string_lossy().to_string(),
                process.cpu_usage(),
            )
        })
        .collect();

    processes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top = processes
        .first()
        .map(|(name, _cpu)| {
            let name = name.trim_end_matches('\0');
            if name.is_empty() {
                "Unknown"
            } else {
                name
            }
        })
        .unwrap_or("None");

    let cpu_usage = processes.first().map(|(_, cpu)| *cpu as u32).unwrap_or(0);

    let top_process = format!("{} {}%", top, cpu_usage);

    ProcessInfo {
        top_process,
        cpu_usage,
    }
}