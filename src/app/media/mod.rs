pub mod popup;

use mpris::{Event, PlaybackStatus, PlayerFinder};
use slint::ComponentHandle;
use std::thread;
use std::time::Duration;

fn with_adapter(
    window: &slint::Weak<crate::StatusBarWindow>,
    f: impl FnOnce(&crate::MediaAdapter) + Send + 'static,
) {
    let w = window.clone();
    let _sty = slint::invoke_from_event_loop(move || {
        if let Some(w) = w.upgrade() {
            f(&w.global::<crate::MediaAdapter>());
        }
    });
}

fn update_text(window: &slint::Weak<crate::StatusBarWindow>, metadata: Option<&mpris::Metadata>) {
    let title = metadata
        .and_then(|m| m.title())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let artist = metadata
        .and_then(|m| m.artists())
        .and_then(|a| a.first().copied())
        .map(|s| s.to_string())
        .unwrap_or_default();

    with_adapter(window, move |a| {
        a.set_title(title.into());
        a.set_artist(artist.into());
    });
}

fn update_status(window: &slint::Weak<crate::StatusBarWindow>, status: Option<PlaybackStatus>) {
    let has_player = status.is_some();
    let status_str: String = match status {
        Some(PlaybackStatus::Playing) => "playing".into(),
        Some(PlaybackStatus::Paused) => "paused".into(),
        _ => "stopped".into(),
    };

    with_adapter(window, move |a| {
        a.set_has_player(has_player);
        a.set_status(status_str.into());
    });
}

fn clear_ui(window: &slint::Weak<crate::StatusBarWindow>) {
    with_adapter(window, |a| {
        a.set_has_player(false);
        a.set_status("stopped".into());
        a.set_title("Sin música".into());
        a.set_artist(String::new().into());
    });
}

pub struct MediaController;

impl MediaController {
    pub fn connect(window: &crate::StatusBarWindow) {
        window.global::<crate::MediaAdapter>().on_play_pause(|| {
            log::info!("[media] play-pause not yet implemented");
        });

        window.global::<crate::MediaAdapter>().on_next(|| {
            log::info!("[media] next not yet implemented");
        });

        window.global::<crate::MediaAdapter>().on_previous(|| {
            log::info!("[media] previous not yet implemented");
        });

        let weak = window.as_weak();

        thread::spawn(move || {
            loop {
                let finder = match PlayerFinder::new() {
                    Ok(f) => f,
                    Err(e) => {
                        log::warn!("[media] D-Bus connection failed: {e}");
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                };

                let player = match finder.find_active() {
                    Ok(p) => p,
                    Err(_) => {
                        clear_ui(&weak);
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };

                log::info!("[media] found active player");

                update_text(&weak, player.get_metadata().ok().as_ref());
                update_status(&weak, player.get_playback_status().ok());

                let events = match player.events() {
                    Ok(e) => e,
                    Err(e) => {
                        log::error!("[media] failed to create event listener: {e}");
                        continue;
                    }
                };

                for result in events {
                    match result {
                        Ok(Event::PlayerShutDown) => {
                            log::info!("[media] player shutting down");
                            break;
                        }
                        Ok(Event::TrackChanged(ref meta)) => {
                            update_text(&weak, Some(meta));
                            update_status(&weak, player.get_playback_status().ok());
                        }
                        Ok(Event::Playing) => update_status(&weak, Some(PlaybackStatus::Playing)),
                        Ok(Event::Paused) => update_status(&weak, Some(PlaybackStatus::Paused)),
                        Ok(Event::Stopped) => update_status(&weak, None),
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("[media] event error: {e}");
                            break;
                        }
                    }
                }

                clear_ui(&weak);
            }
        });
    }
}
