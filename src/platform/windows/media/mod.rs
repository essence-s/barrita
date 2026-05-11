#![allow(unused_imports)]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSession,
};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum MediaEvent {
    SessionsChanged,
    SessionCreated(String),
    CurrentSessionChanged(String),
    PlaybackChanged(String),
    TimelineChanged(String),
    MediaPropertiesChanged(String),
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
                        cb_all(MediaEvent::SessionCreated(app_id.clone()));
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
                        let app_id_tl = app_id.clone();
                        let app_id_med = app_id.clone();

                        let callback_pb = callback_arc.clone();
                        let callback_tl = callback_arc.clone();
                        let callback_med = callback_arc.clone();

                        let playback_token = session.PlaybackInfoChanged(&TypedEventHandler::<
                            GlobalSystemMediaTransportControlsSession,
                            windows::Media::Control::PlaybackInfoChangedEventArgs,
                        >::new(move |_sender, _args| {
                            println!("[Media] *** PlaybackChanged HIT: {}", app_id_pb);
                            callback_pb(MediaEvent::PlaybackChanged(app_id_pb.clone()));
                            Ok(())
                        })).ok();

                        let timeline_token = session.TimelinePropertiesChanged(&TypedEventHandler::<
                            GlobalSystemMediaTransportControlsSession,
                            windows::Media::Control::TimelinePropertiesChangedEventArgs,
                        >::new(move |_sender, _args| {
                            println!("[Media] *** TimelineChanged HIT: {}", app_id_tl);
                            callback_tl(MediaEvent::TimelineChanged(app_id_tl.clone()));
                            Ok(())
                        })).ok();

                        let media_token = session.MediaPropertiesChanged(&TypedEventHandler::<
                            GlobalSystemMediaTransportControlsSession,
                            windows::Media::Control::MediaPropertiesChangedEventArgs,
                        >::new(move |_sender, _args| {
                            println!("[Media] *** MediaPropertiesChanged HIT: {}", app_id_med);
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

pub fn start_media_listener<F>(callback: F) -> Result<MediaListener, Box<dyn std::error::Error>>
where
    F: 'static + Fn(MediaEvent) + Send + Clone + Sync,
{
    MediaListener::new(callback)
}