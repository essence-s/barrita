mod mpris;

#[cfg(target_os = "windows")]
mod windows;
// pub use mpris::MediaPlayerInfo;
#[cfg(target_os = "linux")]
pub use mpris::{get_media_info, next, play_pause, previous};

#[cfg(target_os = "windows")]
pub use windows::{get_media_info, next, play_pause, previous};

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use mpris::get_media_info;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn play_pause() -> Result<(), String> {
    Err("Media control not supported on this platform".to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn next() -> Result<(), String> {
    Err("Media control not supported on this platform".to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn previous() -> Result<(), String> {
    Err("Media control not supported on this platform".to_string())
}
