use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
        GlobalSystemMediaTransportControlsSession,
    },
};

#[derive(Debug, Clone)]
pub enum MediaEvent {
    SessionCreated,
    SessionRemoved,
    CurrentSessionChanged,
    SessionsChanged,
    PlaybackInfoChanged,
    TimelinePropertiesChanged,
    MediaPropertiesChanged,
}

#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub status: String,
    pub has_player: bool,
    pub position_secs: u64,
    pub duration_secs: u64,
}

pub struct MediaListener {
    _receiver: Arc<Mutex<Option<mpsc::Receiver<MediaEvent>>>>,
}

impl MediaListener {
    pub fn new<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: 'static + Send + Fn(MediaEvent) + Clone,
    {
        let (_tx, rx) = mpsc::channel::<MediaEvent>();
        let callback_clone = callback.clone();

        thread::spawn(move || {
            run_media_listener_loop(callback_clone);
        });

        Ok(MediaListener {
            _receiver: Arc::new(Mutex::new(Some(rx))),
        })
    }
}

fn run_media_listener_loop<F>(callback: F)
where
    F: 'static + Send + Fn(MediaEvent) + Clone,
{
    println!("[Media] Starting media listener thread...");

    let manager: GlobalSystemMediaTransportControlsSessionManager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
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

    let mut last_session_available = false;

    loop {
        thread::sleep(Duration::from_millis(500));

        let session_result: Result<GlobalSystemMediaTransportControlsSession, windows::core::Error> = manager.GetCurrentSession();

        match session_result {
            Ok(session) => {
                if !last_session_available {
                    println!("[Media] Session created");
                    last_session_available = true;
                    callback(MediaEvent::SessionCreated);
                }

                let playback_info_result = session.GetPlaybackInfo();
                if let Ok(info) = playback_info_result {
                    let status = info.PlaybackStatus().unwrap_or(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped);
                    let status_str = match status {
                        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => "playing",
                        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => "paused",
                        _ => "stopped",
                    };
                    println!("[Media] Playback status: {}", status_str);
                    callback(MediaEvent::PlaybackInfoChanged);
                }

                let timeline_result = session.GetTimelineProperties();
                if timeline_result.is_ok() {
                    println!("[Media] Timeline updated");
                    callback(MediaEvent::TimelinePropertiesChanged);
                }

                let props_async_result = session.TryGetMediaPropertiesAsync();
                if let Ok(op) = props_async_result {
                    if op.join().is_ok() {
                        println!("[Media] Media properties updated");
                        callback(MediaEvent::MediaPropertiesChanged);
                    }
                }
            }
            Err(_) => {
                if last_session_available {
                    println!("[Media] Session removed");
                    last_session_available = false;
                    callback(MediaEvent::SessionRemoved);
                }
            }
        }
    }
}

pub fn start_media_listener<F>(callback: F) -> Result<MediaListener, Box<dyn std::error::Error>>
where
    F: 'static + Send + Fn(MediaEvent) + Clone,
{
    MediaListener::new(callback)
}