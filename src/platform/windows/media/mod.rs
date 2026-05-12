#![allow(unused_imports)]
use std::sync::{Arc, Mutex};
use std::thread;

use windows::Media::MediaPlaybackStatus;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

#[derive(Debug, Clone)]
pub enum MediaUpdate {
    PlaybackStatus(String),
    MediaInfo { 
        title: String, 
        artist: String, 
        status: String, 
        has_player: bool,
        thumbnail: Option<Vec<u8>>,
    },
}

#[derive(Debug, Clone)]
struct MediaState {
    status: String,
    title: String,
    artist: String,
    has_player: bool,
    thumbnail: Vec<u8>,
}

impl Default for MediaState {
    fn default() -> Self {
        MediaState {
            status: "stopped".to_string(),
            title: "Sin música".to_string(),
            artist: String::new(),
            has_player: false,
            thumbnail: Vec::new(),
        }
    }
}

pub struct MediaListener {
    _running: Arc<Mutex<bool>>,
}

fn get_playback_status_internal(manager: &GlobalSystemMediaTransportControlsSessionManager) -> Result<bool, Box<dyn std::error::Error>> {
    let session = match manager.GetCurrentSession() {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };

    let playback_info = session.GetPlaybackInfo()?;
    let playback_status = playback_info.PlaybackStatus()?;
    let is_playing = playback_status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    Ok(is_playing)
}

fn get_media_properties_internal(manager: &GlobalSystemMediaTransportControlsSessionManager) -> Result<(String, String, bool, Option<Vec<u8>>), Box<dyn std::error::Error>> {
    let session = match manager.GetCurrentSession() {
        Ok(s) => s,
        Err(_) => return Ok(("Sin música".to_string(), String::new(), false, None)),
    };

    let playback_info = session.GetPlaybackInfo()?;
    let playback_status = playback_info.PlaybackStatus()?;
    let is_playing = playback_status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    let mut title = String::new();
    let mut artist = String::new();
    let mut thumbnail: Option<Vec<u8>> = None;

    match session.TryGetMediaPropertiesAsync() {
        Ok(async_op) => {
            match async_op.join() {
                Ok(props) => {
                    title = props.Title().map(|s| s.to_string()).unwrap_or_default();
                    artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();

                    if let Ok(thumb_ref) = props.Thumbnail() {
                        match thumb_ref.OpenReadAsync() {
                            Ok(read_op) => {
                                match read_op.join() {
                                    Ok(stream) => {
                                        let size_u64 = stream.Size().unwrap_or(0);
                                        let size = size_u64 as u32;
                                        
                                        if size > 0 && size_u64 < 10_000_000 {
                                            let buffer = windows::Storage::Streams::Buffer::Create(size)?;
                                            match stream.ReadAsync(
                                                &buffer,
                                                size,
                                                windows::Storage::Streams::InputStreamOptions::None
                                            ) {
                                                Ok(read_op) => {
                                                    match read_op.join() {
                                                        Ok(filled_buffer) => {
                                                            let reader = windows::Storage::Streams::DataReader::FromBuffer(&filled_buffer)?;
                                                            let capacity = filled_buffer.Length().unwrap_or(0);
                                                            if capacity > 0 {
                                                                let mut data = vec![0u8; capacity as usize];
                                                                reader.ReadBytes(&mut data)?;
                                                                if !data.is_empty() {
                                                                    thumbnail = Some(data);
                                                                }
                                                            }
                                                        }
                                                        Err(_) => {}
                                                    }
                                                }
                                                Err(_) => {}
                                            }
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
                Err(_) => {}
            }
        }
        Err(_) => {}
    }

    if title.is_empty() {
        title = "Sin música".to_string();
    }

    Ok((title, artist, is_playing, thumbnail))
}

fn send_media_update(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
    state: &mut MediaState,
    callback: &Arc<dyn Fn(MediaUpdate) + Send + Sync>,
) -> bool {
    match get_media_properties_internal(manager) {
        Ok((title, artist, is_playing, thumbnail)) => {
            let has_player = title != "Sin música";
            let status = if is_playing {
                "playing"
            } else if has_player {
                "paused"
            } else {
                "stopped"
            };

            let mut changed = false;

            if state.title != title {
                state.title = title.clone();
                changed = true;
            }
            if state.artist != artist {
                state.artist = artist.clone();
                changed = true;
            }
            if state.status != status {
                state.status = status.to_string();
                changed = true;
            }
            if state.has_player != has_player {
                state.has_player = has_player;
                changed = true;
            }
            
            if let Some(ref new_thumb) = thumbnail {
                if &state.thumbnail != new_thumb {
                    state.thumbnail = new_thumb.clone();
                    changed = true;
                }
            } else if !state.thumbnail.is_empty() {
                state.thumbnail.clear();
                changed = true;
            }

            if changed {
                println!("[Media] Media updated: {} - {} ({})", title, artist, status);
                callback(MediaUpdate::MediaInfo {
                    title,
                    artist,
                    status: status.to_string(),
                    has_player,
                    thumbnail,
                });
            }

            changed
        }
        Err(e) => {
            println!("[Media] Error getting media info: {:?}", e);
            false
        }
    }
}

impl MediaListener {
    pub fn new<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: 'static + Fn(MediaUpdate) + Send + Sync,
    {
        let running = Arc::new(Mutex::new(true));
        let current_state: Arc<Mutex<MediaState>> = Arc::new(Mutex::new(MediaState::default()));

        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;
        let manager_for_initial = manager.clone();

        let callback_arc: Arc<dyn Fn(MediaUpdate) + Send + Sync> = Arc::new(callback);
        let state_clone = current_state.clone();

        println!("[Media] TypedEventHandler registered, scanning initial sessions...");

        // Cargar info inicial
        {
            let mut state = state_clone.lock().unwrap();
            send_media_update(&manager_for_initial, &mut state, &callback_arc);
        }

        if let Ok(sessions_list) = manager_for_initial.GetSessions() {
            let count = sessions_list.Size().unwrap_or(0);
            if count > 0 {
                println!("[Media] Found {} initial sessions", count);

                for i in 0..count {
                    if let Ok(session) = sessions_list.GetAt(i) {
                        let app_id = session.SourceAppUserModelId()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        println!("[Media] Registering handlers for: {}", app_id);

                        let callback_pb = callback_arc.clone();
                        let state_pb = state_clone.clone();
                        let manager_pb = manager_for_initial.clone();

                        let playback_token = session.PlaybackInfoChanged(&TypedEventHandler::<
                            GlobalSystemMediaTransportControlsSession,
                            windows::Media::Control::PlaybackInfoChangedEventArgs,
                        >::new(move |_sender, _args| {
                            match get_playback_status_internal(&manager_pb) {
                                Ok(is_playing) => {
                                    let new_status = if is_playing { "playing" } else { "paused" };
                                    let mut state = state_pb.lock().unwrap();
                                    if state.status != new_status {
                                        println!("[Media] Playback changed: {} -> {}", state.status, new_status);
                                        state.status = new_status.to_string();
                                        callback_pb(MediaUpdate::PlaybackStatus(new_status.to_string()));
                                    }
                                }
                                Err(e) => {
                                    println!("[Media] Error getting playback status: {:?}", e);
                                }
                            }
                            Ok(())
                        })).ok();

                        let callback_med = callback_arc.clone();
                        let state_med = state_clone.clone();
                        let manager_med = manager_for_initial.clone();

                        let media_token = session.MediaPropertiesChanged(&TypedEventHandler::<
                            GlobalSystemMediaTransportControlsSession,
                            windows::Media::Control::MediaPropertiesChangedEventArgs,
                        >::new(move |_sender, _args| {
                            let mut state = state_med.lock().unwrap();
                            send_media_update(&manager_med, &mut state, &callback_med);
                            Ok(())
                        })).ok();

                        let _ = playback_token;
                        let _ = media_token;
                    }
                }
            } else {
                println!("[Media] No initial sessions found");
            }
        }

        let running_clone = running.clone();
        std::thread::spawn(move || {
            while *running_clone.lock().unwrap() {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        });

        Ok(MediaListener {
            _running: running,
        })
    }
}

pub fn start_media_listener<F>(callback: F) -> Result<MediaListener, Box<dyn std::error::Error>>
where
    F: 'static + Fn(MediaUpdate) + Send + Clone + Sync,
{
    MediaListener::new(callback)
}