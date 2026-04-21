pub mod windows;

#[cfg(target_os = "windows")]
pub use windows::get_top_process;

#[cfg(not(target_os = "windows"))]
pub fn get_top_process() -> crate::core::data::ProcessInfo {
    crate::core::data::ProcessInfo {
        top_process: "N/A".to_string(),
        cpu_usage: 0,
    }
}