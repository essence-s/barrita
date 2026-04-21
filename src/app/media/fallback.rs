use crate::core::data::MediaPlayerInfo;

#[allow(dead_code)]
pub fn get_media_info() -> Option<MediaPlayerInfo> {
    None
}

#[allow(dead_code)]
pub fn play_pause() -> Result<(), String> {
    Err("Media control not supported on this platform".to_string())
}

#[allow(dead_code)]
pub fn next() -> Result<(), String> {
    Err("Media control not supported on this platform".to_string())
}

#[allow(dead_code)]
pub fn previous() -> Result<(), String> {
    Err("Media control not supported on this platform".to_string())
}