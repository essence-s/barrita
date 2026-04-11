use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaPlayerInfo {
    pub title: String,
    pub artist: String,
    pub status: String,
    pub has_player: bool,
    pub album_art: Vec<u8>,
    pub progress: f32,
    pub progress_time: String,
    pub total_time: String,
}

static MEDIA_CACHE: Mutex<Option<MediaPlayerInfo>> = Mutex::new(None);
static LAST_ATTEMPT_TIME: std::sync::LazyLock<Mutex<Instant>> =
    std::sync::LazyLock::new(|| Mutex::new(Instant::now()));

fn is_valid_image_format(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let first_bytes: [u8; 4] = [data[0], data[1], data[2], data[3]];
    first_bytes == [0x89, 0x50, 0x4E, 0x47]
        || (data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF)
        || first_bytes == [0x42, 0x4D, 0x00, 0x00]
}

fn get_cached_or_default() -> MediaPlayerInfo {
    if let Ok(cache) = MEDIA_CACHE.lock() {
        if let Some(info) = cache.as_ref() {
            return info.clone();
        }
    }
    MediaPlayerInfo {
        title: "Sin música".to_string(),
        artist: String::new(),
        status: "stopped".to_string(),
        has_player: false,
        album_art: Vec::new(),
        progress: 0.0,
        progress_time: "0:00".to_string(),
        total_time: "0:00".to_string(),
    }
}

#[cfg(target_os = "windows")]
pub fn get_media_info() -> Option<MediaPlayerInfo> {
    let cooldown = Duration::from_secs(10);
    let now = Instant::now();

    // Check cooldown
    let should_attempt = {
        if let Ok(last_attempt) = LAST_ATTEMPT_TIME.lock() {
            last_attempt.elapsed() > cooldown
        } else {
            true
        }
    };

    // Return cached if within cooldown
    if !should_attempt {
        return Some(get_cached_or_default());
    }

    // Try to get fresh data in a thread with timeout
    let result = std::thread::spawn(|| get_media_info_internal())
        .join()
        .ok()
        .flatten();

    // Update cache and timestamp
    if let Some(ref info) = result {
        if let Ok(mut cache) = MEDIA_CACHE.lock() {
            *cache = Some(info.clone());
        }
    }

    if let Ok(mut last_attempt) = LAST_ATTEMPT_TIME.lock() {
        *last_attempt = now;
    }

    result.or_else(|| Some(get_cached_or_default()))
}

#[cfg(target_os = "windows")]
fn get_media_info_internal() -> Option<MediaPlayerInfo> {
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;

    let start = Instant::now();
    let timeout_total = Duration::from_millis(150);
    let timeout_thumbnail = Duration::from_millis(30);

    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => match op.get() {
            Ok(m) => m,
            Err(_) => return None,
        },
        Err(_) => return None,
    };

    if start.elapsed() > timeout_total {
        return None;
    }

    let session = match manager.GetCurrentSession() {
        Ok(s) => s,
        Err(_) => return None,
    };

    let mut title_str = String::new();
    let mut artist_str = String::new();
    let mut album_art: Vec<u8> = Vec::new();

    let props = match session.TryGetMediaPropertiesAsync() {
        Ok(op) => match op.get() {
            Ok(p) => p,
            Err(_) => return None,
        },
        Err(_) => return None,
    };

    if start.elapsed() > timeout_total {
        return None;
    }

    title_str = props.Title().unwrap_or_default().to_string();
    artist_str = props.Artist().unwrap_or_default().to_string();

    if start.elapsed() < timeout_total - Duration::from_millis(20) {
        if let Ok(thumbnail) = props.Thumbnail() {
            if start.elapsed() < timeout_thumbnail {
                if let Ok(async_op) = thumbnail.OpenReadAsync() {
                    if start.elapsed() < timeout_thumbnail {
                        if let Ok(read_stream) = async_op.get() {
                            if let Ok(size) = read_stream.Size() {
                                let size_u32 = size as u32;
                                if size_u32 > 100 && size_u32 < 2_000_000 {
                                    if let Ok(reader) = DataReader::CreateDataReader(&read_stream) {
                                        if let Ok(load_op) = reader.LoadAsync(size_u32) {
                                            if load_op.get().is_ok() {
                                                let mut buffer = vec![0u8; size_u32 as usize];
                                                let _ = reader.ReadBytes(&mut buffer);

                                                if is_valid_image_format(&buffer) {
                                                    log::info!(
                                                        "Thumbnail valid: {} bytes",
                                                        buffer.len()
                                                    );
                                                    album_art = buffer;
                                                } else {
                                                    log::warn!("Thumbnail invalid format");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout_total {
        return None;
    }

    let playback_info = match session.GetPlaybackInfo() {
        Ok(p) => p,
        Err(_) => return None,
    };

    let status_str = match playback_info.PlaybackStatus() {
        Ok(s) => match s {
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => "playing",
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => "paused",
            _ => "stopped",
        },
        Err(_) => "stopped",
    };

    let timeline = match session.GetTimelineProperties() {
        Ok(t) => t,
        Err(_) => return None,
    };

    let position = match timeline.Position() {
        Ok(p) => p,
        Err(_) => return None,
    };
    let duration = match timeline.EndTime() {
        Ok(d) => d,
        Err(_) => return None,
    };

    let position_secs = position.Duration / 10_000_000;
    let duration_secs = duration.Duration / 10_000_000;

    let progress = if duration_secs > 0 {
        (position_secs as f32) / (duration_secs as f32)
    } else {
        0.0
    };

    let progress_time = format_time(position_secs as u64);
    let total_time = format_time(duration_secs as u64);

    let has_active_player = !title_str.is_empty() || status_str == "playing";

    Some(MediaPlayerInfo {
        title: if title_str.is_empty() {
            "Sin música".to_string()
        } else {
            title_str
        },
        artist: artist_str,
        status: status_str.to_string(),
        has_player: has_active_player,
        album_art,
        progress,
        progress_time,
        total_time,
    })
}

fn format_time(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", mins, secs)
}

#[cfg(target_os = "windows")]
pub fn play_pause() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};

    unsafe {
        keybd_event(0xB3, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(0xB3, 0, KEYBD_EVENT_FLAGS(2), 0);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn next() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};

    unsafe {
        keybd_event(0xB0, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(0xB0, 0, KEYBD_EVENT_FLAGS(2), 0);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn previous() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};

    unsafe {
        keybd_event(0xB1, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(0xB1, 0, KEYBD_EVENT_FLAGS(2), 0);
    }
    Ok(())
}
