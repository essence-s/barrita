use crate::core::data::MediaPlayerInfo;

#[cfg(target_os = "linux")]
pub fn get_media_info() -> Option<MediaPlayerInfo> {
    use mpris::{Player, PlayerFinder};

    let finder = PlayerFinder::new().ok()?;
    let player = finder.find_active_player().ok()?;

    let title = player
        .get_metadata()
        .ok()
        .and_then(|m| m.title)
        .unwrap_or_else(|| "Unknown".to_string());

    let artist = player
        .get_metadata()
        .ok()
        .and_then(|m| m.artists)
        .and_then(|v| v.first().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let status = match player.get_playback_status().ok() {
        Some(mpris::PlaybackStatus::Playing) => "playing",
        Some(mpris::PlaybackStatus::Paused) => "paused",
        Some(mpris::PlaybackStatus::Stopped) => "stopped",
        None => "stopped",
    };

    Some(MediaPlayerInfo {
        title,
        artist,
        status: status.to_string(),
        has_player: true,
        album_art: Vec::new(),
        progress: 0.0,
        progress_time: "0:00".to_string(),
        total_time: "0:00".to_string(),
    })
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
pub fn get_media_info() -> Option<MediaPlayerInfo> {
    None
}

#[cfg(target_os = "linux")]
pub fn play_pause() -> Result<(), String> {
    use mpris::{Player, PlayerFinder};

    let finder = PlayerFinder::new().map_err(|e| e.to_string())?;
    let player = finder.find_active_player().map_err(|e| e.to_string())?;

    player.play_pause().map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
pub fn next() -> Result<(), String> {
    use mpris::{Player, PlayerFinder};

    let finder = PlayerFinder::new().map_err(|e| e.to_string())?;
    let player = finder.find_active_player().map_err(|e| e.to_string())?;

    player.next().map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
pub fn previous() -> Result<(), String> {
    use mpris::{Player, PlayerFinder};

    let finder = PlayerFinder::new().map_err(|e| e.to_string())?;
    let player = finder.find_active_player().map_err(|e| e.to_string())?;

    player.previous().map_err(|e| e.to_string())
}