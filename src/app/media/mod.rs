#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub mod fallback;

#[cfg(target_os = "linux")]
pub use linux::{get_media_info, next, play_pause, previous};

#[cfg(target_os = "windows")]
pub use windows::{get_media_info, next, play_pause, previous};

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use fallback::{get_media_info, next, play_pause, previous};