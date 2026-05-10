use std::collections::HashMap;
use std::thread;
use std::sync::{Arc, Mutex};

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum MediaEvent {
    SessionCreated(()),
    SessionRemoved(()),
    CurrentSessionChanged(Option<String>),
    PlaybackInfoChanged(()),
    MediaPropertiesChanged(()),
    TimelinePropertiesChanged(()),
}

pub struct MediaListener {
    _running: Arc<Mutex<bool>>,
}

impl MediaListener {
    pub fn new<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: 'static + Fn(MediaEvent) + Send + Clone,
    {
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();

        thread::spawn(move || {
            run_event_loop(callback, running_clone);
        });

        Ok(MediaListener {
            _running: running,
        })
    }
}

fn run_event_loop<F>(callback: F, running: Arc<Mutex<bool>>)
where
    F: 'static + Fn(MediaEvent) + Send + Clone,
{
    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => match op.join() {
            Ok(m) => {
                println!("[Media] Manager initialized");
                m
            }
            Err(e) => {
                println!("[Media] Failed to get manager: {:?}", e);
                return;
            }
        },
        Err(e) => {
            println!("[Media] RequestAsync failed: {:?}", e);
            return;
        }
    };

    let mut last_session_id: Option<String> = None;
    let mut known_sessions: HashMap<String, bool> = HashMap::new();

    if let Ok(sessions) = manager.GetSessions() {
        for session in sessions {
            let id = session.SourceAppUserModelId().unwrap_or_default().to_string();
            known_sessions.insert(id.clone(), true);
            println!("[Media] Session created: {}", id);
            callback(MediaEvent::SessionCreated(()));
        }
    }

    if let Ok(session) = manager.GetCurrentSession() {
        let id = session.SourceAppUserModelId().unwrap_or_default().to_string();
        last_session_id = Some(id.clone());
        callback(MediaEvent::CurrentSessionChanged(Some(id)));
    }

    while *running.lock().unwrap() {
        let new_sessions = manager.GetSessions();
        
        if let Ok(sessions) = new_sessions {
            let mut current_ids: Vec<String> = Vec::new();
            
            for session in sessions {
                let id = session.SourceAppUserModelId().unwrap_or_default().to_string();
                current_ids.push(id.clone());
                
                if !known_sessions.contains_key(&id) {
                    println!("[Media] Session created: {}", id);
                    known_sessions.insert(id.clone(), true);
                    callback(MediaEvent::SessionCreated(()));
                }
            }

            let ids_to_remove: Vec<String> = known_sessions.keys()
                .filter(|id| !current_ids.contains(id))
                .cloned()
                .collect();
            
            for id in ids_to_remove {
                println!("[Media] Session removed: {}", id);
                known_sessions.remove(&id);
                callback(MediaEvent::SessionRemoved(()));
            }
        }

        if let Ok(session) = manager.GetCurrentSession() {
            let current_id = session.SourceAppUserModelId().unwrap_or_default().to_string();
            if last_session_id.as_ref() != Some(&current_id) {
                println!("[Media] Current session changed: {}", current_id);
                last_session_id = Some(current_id.clone());
                callback(MediaEvent::CurrentSessionChanged(Some(current_id)));
            }
        } else if last_session_id.is_some() {
            println!("[Media] Current session: None");
            last_session_id = None;
            callback(MediaEvent::CurrentSessionChanged(None));
        }

        thread::sleep(std::time::Duration::from_millis(500));
    }

    println!("[Media] Event loop ended");
}

pub fn start_media_listener<F>(callback: F) -> Result<MediaListener, Box<dyn std::error::Error>>
where
    F: 'static + Fn(MediaEvent) + Send + Clone,
{
    MediaListener::new(callback)
}