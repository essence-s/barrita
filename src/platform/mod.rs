#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

pub mod tray;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub mod linux;