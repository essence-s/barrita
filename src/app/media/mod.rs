use dbus::ffidisp::{BusType, Connection, ConnectionItem};
use mpris::{PlaybackStatus, PlayerFinder};
use slint::ComponentHandle;
use backend::wayland_adapter::WinHandle;
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
    pub fn connect(window: &crate::StatusBarWindow, ctrl_handler: WinHandle) {
        window.global::<crate::MediaAdapter>().on_toggle_control_center(move || {
            ctrl_handler.toggle();
        });

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
                let conn = match Connection::get_private(BusType::Session) {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("[media] D-Bus connection failed: {e}");
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                };

                let _ = conn.add_match(
                    "interface='org.freedesktop.DBus',\
                     member='NameOwnerChanged',\
                     arg0namespace='org.mpris.MediaPlayer2'",
                );

                let _ = conn.add_match(
                    "interface='org.freedesktop.DBus.Properties',\
                     member='PropertiesChanged',\
                     path='/org/mpris/MediaPlayer2'",
                );

                let finder = match PlayerFinder::new() {
                    Ok(f) => f,
                    Err(e) => {
                        log::warn!("[media] D-Bus finder failed: {e}");
                        continue;
                    }
                };

                for item in conn.iter(1000) {
                    if let ConnectionItem::Signal(msg) = &item {
                        log::debug!(
                            "[media] signal: {}.{}",
                            msg.interface().as_deref().unwrap_or("?"),
                            msg.member().as_deref().unwrap_or("?")
                        );
                    }

                    match finder.find_active() {
                        Ok(player) => {
                            update_text(&weak, player.get_metadata().ok().as_ref());
                            update_status(&weak, player.get_playback_status().ok());
                        }
                        Err(_) => {
                            clear_ui(&weak);
                        }
                    }
                }

                log::info!("[media] D-Bus connection lost, reconnecting...");
            }
        });
    }
}
