use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaPlayerInfo {
    pub title: String,
    pub artist: String,
    pub status: String,
    pub has_player: bool,
}

#[cfg(target_os = "windows")]
pub fn get_media_info() -> Option<MediaPlayerInfo> {
    Some(MediaPlayerInfo {
        title: "Sin información".to_string(),
        artist: "".to_string(),
        status: "stopped".to_string(),
        has_player: true,
    })
}

#[cfg(target_os = "windows")]
pub fn play_pause() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};

    unsafe {
        // Media Play/Pause: 0xB3
        keybd_event(0xB3, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(0xB3, 0, KEYBD_EVENT_FLAGS(2), 0);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn next() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};

    unsafe {
        // Media Next: 0xB0
        keybd_event(0xB0, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(0xB0, 0, KEYBD_EVENT_FLAGS(2), 0);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn previous() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};

    unsafe {
        // Media Previous: 0xB1
        keybd_event(0xB1, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(0xB1, 0, KEYBD_EVENT_FLAGS(2), 0);
    }
    Ok(())
}
