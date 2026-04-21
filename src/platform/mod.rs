#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub mod linux;

pub use self::windows::{init_statusbar, AppBarEdge, StatusBarConfig};