use dbus::ffidisp::{BusType, Connection, ConnectionItem};
use mpris::{PlaybackStatus, PlayerFinder};
use slint::ComponentHandle;
use std::thread;
use std::time::Duration;

/// Snapshot completo del player activo. Sirve para hacer diff y solo
/// tocar la UI cuando algo realmente cambió (título, artista o estado).
#[derive(Clone, PartialEq, Eq, Default)]
struct MediaState {
    has_player: bool,
    status: String,
    title: String,
    artist: String,
}

impl MediaState {
    fn empty() -> Self {
        Self {
            has_player: false,
            status: "stopped".into(),
            title: "Sin música".into(),
            artist: String::new(),
        }
    }
}

fn push_state(window: &slint::Weak<crate::StatusBarWindow>, state: &MediaState) {
    let state = state.clone();
    let w = window.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(w) = w.upgrade() {
            let a = w.global::<crate::MediaAdapter>();
            a.set_has_player(state.has_player);
            a.set_status(state.status.into());
            a.set_title(state.title.into());
            a.set_artist(state.artist.into());
        }
    });
}

/// Lee el player activo (Playing > Paused > con track > primero).
/// `find_active` ya implementa esa prioridad, que es justo lo que se
/// quiere con múltiples apps de audio (Spotify + Firefox + VLC...).
fn read_active(finder: &PlayerFinder) -> MediaState {
    let player = match finder.find_active() {
        Ok(p) => p,
        Err(_) => return MediaState::empty(),
    };

    let (title, artist) = match player.get_metadata() {
        Ok(m) => (
            m.title().map(|s| s.to_string()).unwrap_or_default(),
            m.artists()
                .and_then(|a| a.first().copied())
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ),
        Err(_) => (String::new(), String::new()),
    };

    let (has_player, status) = match player.get_playback_status() {
        Ok(PlaybackStatus::Playing) => (true, "playing"),
        Ok(PlaybackStatus::Paused) => (true, "paused"),
        _ => (true, "stopped"),
    };

    // Player existe pero sin metadata ni reproducción: mostrar estado vacío
    // en vez de strings vacíos que parpadean.
    if title.is_empty() && artist.is_empty() && status == "stopped" {
        return MediaState::empty();
    }

    MediaState {
        has_player,
        status: status.into(),
        title,
        artist,
    }
}

/// Decide si una señal D-Bus merece re-leer el estado.
/// - NameOwnerChanged (namespace MPRIS): aparece/desaparece un player.
/// - PropertiesChanged en Player: metadata o estado. Se filtran TrackList
///   y otras interfaces para no consultar en cambios de volumen/seek.
fn should_refresh(msg: &dbus::Message) -> bool {
    let member: String = msg.member().map(|m| m.to_string()).unwrap_or_default();
    let iface: String = msg.interface().map(|i| i.to_string()).unwrap_or_default();

    if member == "NameOwnerChanged" && iface == "org.freedesktop.DBus" {
        return true;
    }

    if member == "PropertiesChanged" && iface == "org.freedesktop.DBus.Properties" {
        // arg0 = interfaz que cambió ("org.mpris.MediaPlayer2.Player", ...)
        match msg.get1::<String>() {
            Some(changed_iface) => {
                return changed_iface == "org.mpris.MediaPlayer2.Player";
            }
            // Si no se puede parsear, refrescar por seguridad.
            None => return true,
        }
    }

    false
}

/// Refresca una vez y pushea a UI solo si cambió. Retorna el estado actual.
fn refresh_if_changed(
    finder: &PlayerFinder,
    window: &slint::Weak<crate::StatusBarWindow>,
    last: &mut MediaState,
    initialized: &mut bool,
) {
    let current = read_active(finder);
    if !*initialized || current != *last {
        *last = current.clone();
        *initialized = true;
        log::info!(
            "[media] {} — {} | {}",
            if current.has_player {
                current.status.as_str()
            } else {
                "sin player"
            },
            current.artist,
            current.title
        );
        push_state(window, &current);
    }
}

fn control_action(action: &str) {
    let action = action.to_string();
    // D-Bus bloquea (hasta ~500ms): nunca en el hilo UI de Slint.
    thread::spawn(move || {
        let finder = match PlayerFinder::new() {
            Ok(f) => f,
            Err(e) => {
                log::warn!("[media] control {action}: sin D-Bus: {e}");
                return;
            }
        };
        let player = match finder.find_active() {
            Ok(p) => p,
            Err(_) => {
                log::debug!("[media] control {action}: sin player activo");
                return;
            }
        };
        let res = match action.as_str() {
            "play-pause" => player.play_pause(),
            "next" => player.next(),
            "previous" => player.previous(),
            _ => return,
        };
        if let Err(e) = res {
            log::warn!("[media] control {action} falló: {e}");
        }
    });
}

pub struct MediaController;

impl MediaController {
    pub fn connect(window: &crate::StatusBarWindow) {
        window
            .global::<crate::MediaAdapter>()
            .on_play_pause(|| control_action("play-pause"));
        window
            .global::<crate::MediaAdapter>()
            .on_next(|| control_action("next"));
        window
            .global::<crate::MediaAdapter>()
            .on_previous(|| control_action("previous"));

        let weak = window.as_weak();

        thread::spawn(move || loop {
            let conn = match Connection::get_private(BusType::Session) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("[media] D-Bus connection failed: {e}");
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };

            if let Err(e) = conn.add_match(
                "interface='org.freedesktop.DBus',\
                 member='NameOwnerChanged',\
                 arg0namespace='org.mpris.MediaPlayer2'",
            ) {
                log::error!("[media] add_match NameOwnerChanged failed: {e}");
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            if let Err(e) = conn.add_match(
                "interface='org.freedesktop.DBus.Properties',\
                 member='PropertiesChanged',\
                 path='/org/mpris/MediaPlayer2'",
            ) {
                log::error!("[media] add_match PropertiesChanged failed: {e}");
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            // Segunda conexión (interna del finder) solo para queries.
            // Como ya no hay polling, su coste en idle es cero.
            let finder = match PlayerFinder::new() {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("[media] D-Bus finder failed: {e}");
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };

            // Sync inicial: la barra muestra el estado real al arrancar.
            let mut last = MediaState::default();
            let mut initialized = false;
            refresh_if_changed(&finder, &weak, &mut last, &mut initialized);

            log::info!("[media] escuchando eventos MPRIS (sin polling)");

            // `iter(-1)` bloquea indefinido en libdbus hasta que llega
            // una señal: 0 wakeups/CPU en idle, a diferencia de `iter(1000)`
            // que despertaba cada segundo aunque no hubiera cambios.
            for item in conn.iter(-1) {
                if let ConnectionItem::Signal(msg) = &item
                    && should_refresh(msg)
                {
                    refresh_if_changed(&finder, &weak, &mut last, &mut initialized);
                }
                // Nothing/MethodReturn/etc: ignorar sin consultar D-Bus.
            }

            log::info!("[media] D-Bus connection lost, reconnecting...");
            thread::sleep(Duration::from_secs(1));
        });
    }
}
