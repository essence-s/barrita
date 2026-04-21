#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(target_os = "windows"))]
pub mod common;

#[cfg(target_os = "windows")]
pub use windows::get_battery_info;

#[cfg(not(target_os = "windows"))]
pub use common::get_battery_info;