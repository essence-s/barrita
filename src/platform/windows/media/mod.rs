#![allow(unused_imports)]
use std::collections::HashMap;
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
pub struct MediaInfo {
    pub app_id: String,
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
}

impl Default for MediaInfo {
    fn default() -> Self {
        MediaInfo {
            app_id: String::new(),
            is_playing: false,
            title: "Sin música".to_string(),
            artist: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum MediaEvent {
    SessionsChanged,
    SessionCreated(String),
    CurrentSessionChanged(String),
    PlaybackChanged(String),
    TimelineChanged(String),
    MediaPropertiesChanged(String),
    MediaInfoChanged(MediaInfo),
}

#[allow(dead_code)]
struct SessionState {
    session: GlobalSystemMediaTransportControlsSession,
    playback_token: Option<i64>,
    timeline_token: Option<i64>,
    media_token: Option<i64>,
}

pub struct MediaListener {
    _running: Arc<Mutex<bool>>,
    _states: Arc<Mutex<HashMap<String, SessionState>>>,
}

impl MediaListener {
    pub fn new<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: 'static + Fn(MediaEvent) + Send + Clone + Sync,
    {
        let running = Arc::new(Mutex::new(true));
        let states: Arc<Mutex<HashMap<String, SessionState>>> = Arc::new(Mutex::new(HashMap::new()));

        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;
        let manager_for_sessions = manager.clone();
        let manager_for_current = manager.clone();
        let manager_for_initial = manager.clone();

        let callback_arc = Arc::new(callback);
        let cb_all = callback_arc.clone();

        let sessions_handler = TypedEventHandler::<
            GlobalSystemMediaTransportControlsSessionManager,
            windows::Media::Control::SessionsChangedEventArgs,
        >::new(move |_sender, _args| {
            if let Ok(sessions_list) = manager_for_sessions.GetSessions() {
                let count = sessions_list.Size().unwrap_or(0);
                println!("[Media] SessionsChanged event: {} sessions", count);
                cb_all(MediaEvent::SessionsChanged);

                for i in 0..count {
                    if let Ok(session) = sessions_list.GetAt(i) {
                        let app_id = session.SourceAppUserModelId()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        println!("[Media]   New session: {}", app_id);
                        cb_all(MediaEvent::SessionCreated(app_id));
                    }
                }
            }
            Ok(())
        });

        let cb_current = callback_arc.clone();
        let manager_for_current2 = manager_for_current.clone();
        let current_handler = TypedEventHandler::<
            GlobalSystemMediaTransportControlsSessionManager,
            windows::Media::Control::CurrentSessionChangedEventArgs,
        >::new(move |_sender, _args| {
            match manager_for_current2.GetCurrentSession() {
                Ok(session) => {
                    let app_id = session.SourceAppUserModelId()
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    println!("[Media] CurrentSessionChanged: {}", app_id);
                    cb_current(MediaEvent::CurrentSessionChanged(app_id));
                }
                Err(_) => {
                    println!("[Media] CurrentSessionChanged: None");
                    cb_current(MediaEvent::CurrentSessionChanged(String::new()));
                }
            }
            Ok(())
        });

        let _sessions_token = manager.SessionsChanged(&sessions_handler)?;
        let _current_token = manager_for_current.CurrentSessionChanged(&current_handler)?;

        println!("[Media] TypedEventHandler registered, scanning initial sessions...");

        if let Ok(sessions_list) = manager_for_initial.GetSessions() {
            let count = sessions_list.Size().unwrap_or(0);
            if count > 0 {
                println!("[Media] Found {} initial sessions", count);
                let mut states_map = states.lock().unwrap();

                for i in 0..count {
                    if let Ok(session) = sessions_list.GetAt(i) {
                        let app_id = session.SourceAppUserModelId()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        println!("[Media] Registering handlers for: {}", app_id);

                        let app_id_pb = app_id.clone();
                        let app_id_med = app_id.clone();

                        let callback_pb = callback_arc.clone();
                        let callback_med = callback_arc.clone();

                        let playback_token = session.PlaybackInfoChanged(&TypedEventHandler::<
                            GlobalSystemMediaTransportControlsSession,
                            windows::Media::Control::PlaybackInfoChangedEventArgs,
                        >::new(move |_sender, _args| {
                            callback_pb(MediaEvent::PlaybackChanged(app_id_pb.clone()));
                            Ok(())
                        })).ok();

                        // Not using TimelineChanged - too noisy (fires constantly during playback)
                        let timeline_token: Option<i64> = None;

                        let media_token = session.MediaPropertiesChanged(&TypedEventHandler::<
                            GlobalSystemMediaTransportControlsSession,
                            windows::Media::Control::MediaPropertiesChangedEventArgs,
                        >::new(move |_sender, _args| {
                            callback_med(MediaEvent::MediaPropertiesChanged(app_id_med.clone()));
                            Ok(())
                        })).ok();

                        states_map.insert(app_id, SessionState {
                            session,
                            playback_token,
                            timeline_token,
                            media_token,
                        });
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
            _states: states,
        })
    }
}

pub fn get_current_media_info() -> Result<MediaInfo, Box<dyn std::error::Error>> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;

    let session = match manager.GetCurrentSession() {
        Ok(s) => s,
        Err(_) => return Ok(MediaInfo::default()),
    };

    let app_id = session.SourceAppUserModelId()
        .map(|s| s.to_string())
        .unwrap_or_default();

    if app_id.is_empty() {
        return Ok(MediaInfo::default());
    }

    let playback_info = session.GetPlaybackInfo()?;
    let playback_status = playback_info.PlaybackStatus()?;
    let is_playing = playback_status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    let mut title = String::new();
    let mut artist = String::new();

    match session.TryGetMediaPropertiesAsync() {
        Ok(async_op) => {
            if let Ok(props) = async_op.join() {
                title = props.Title().map(|s| s.to_string()).unwrap_or_default();
                artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();
            }
        }
        Err(_) => {}
    }

    if title.is_empty() {
        title = "Sin música".to_string();
    }

    Ok(MediaInfo {
        app_id,
        is_playing,
        title,
        artist,
    })
}

pub fn start_media_listener<F>(callback: F) -> Result<MediaListener, Box<dyn std::error::Error>>
where
    F: 'static + Fn(MediaEvent) + Send + Clone + Sync,
{
    MediaListener::new(callback)
}