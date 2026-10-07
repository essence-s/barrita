use dbus::blocking::Connection;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub mod types;

use types::{ConnectionKind, NetworkInfo};

const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const NM_IFACE: &str = "org.freedesktop.NetworkManager";
const ACTIVE_CONN_IFACE: &str = "org.freedesktop.NetworkManager.Connection.Active";
const DEVICE_IFACE: &str = "org.freedesktop.NetworkManager.Device";

/// NM_STATE_CONNECTED_GLOBAL — hay ruta por defecto E internet (check OK).
const NM_STATE_CONNECTED_GLOBAL: u32 = 70;
const NM_DEVICE_TYPE_ETHERNET: u32 = 1;
const NM_DEVICE_TYPE_WIFI: u32 = 2;

const RECONNECT_DELAY: Duration = Duration::from_secs(10);
const RESYNC_INTERVAL: Duration = Duration::from_secs(30);
const DBUS_TIMEOUT: Duration = Duration::from_secs(5);

fn kind_from_primary_type(primary_type: &str) -> Option<ConnectionKind> {
    match primary_type {
        "802-11-wireless" => Some(ConnectionKind::Wifi),
        "802-3-ethernet" => Some(ConnectionKind::Ethernet),
        _ => None,
    }
}

fn kind_from_device_type(device_type: u32) -> ConnectionKind {
    match device_type {
        NM_DEVICE_TYPE_ETHERNET => ConnectionKind::Ethernet,
        NM_DEVICE_TYPE_WIFI => ConnectionKind::Wifi,
        _ => ConnectionKind::None,
    }
}

/// Fallback: resuelve el tipo desde el primer dispositivo de la conexión activa.
/// Solo se usa cuando `PrimaryConnectionType` no es wifi/ethernet conocido.
fn read_active_device_kind(conn: &Connection, active_path: &str) -> ConnectionKind {
    let proxy = conn.with_proxy(NM_SERVICE, active_path, DBUS_TIMEOUT);
    let devices: Vec<dbus::Path<'static>> = match proxy.get(ACTIVE_CONN_IFACE, "Devices") {
        Ok(d) => d,
        Err(e) => {
            log::debug!("[network] cannot read ActiveConnection devices: {e}");
            return ConnectionKind::None;
        }
    };
    let Some(first) = devices.first() else {
        return ConnectionKind::None;
    };
    let dev_path = first.to_string();
    let dev_proxy = conn.with_proxy(NM_SERVICE, dev_path.as_str(), DBUS_TIMEOUT);
    match dev_proxy.get::<u32>(DEVICE_IFACE, "DeviceType") {
        Ok(t) => kind_from_device_type(t),
        Err(e) => {
            log::debug!("[network] cannot read DeviceType: {e}");
            ConnectionKind::None
        }
    }
}

fn read_network(conn: &Connection) -> NetworkInfo {
    let proxy = conn.with_proxy(NM_SERVICE, NM_PATH, DBUS_TIMEOUT);

    // `State` es estable entre versiones de NM (70 = GLOBAL = internet real).
    // Estados 50/60 (LOCAL/SITE: solo LAN o portal cautivo) cuentan como sin internet.
    let state: u32 = proxy.get(NM_IFACE, "State").unwrap_or(20);
    let connectivity: u32 = proxy.get(NM_IFACE, "Connectivity").unwrap_or(1);
    let connected = state == NM_STATE_CONNECTED_GLOBAL;

    let kind = if !connected {
        ConnectionKind::None
    } else {
        let primary_type: String = proxy
            .get(NM_IFACE, "PrimaryConnectionType")
            .unwrap_or_default();
        match kind_from_primary_type(&primary_type) {
            Some(k) => k,
            None => {
                let primary: dbus::Path<'static> = match proxy.get(NM_IFACE, "PrimaryConnection")
                {
                    Ok(p) => p,
                    Err(_) => {
                        return NetworkInfo {
                            connected,
                            kind: ConnectionKind::None,
                        };
                    }
                };
                let p = primary.to_string();
                if p == "/" {
                    ConnectionKind::None
                } else {
                    read_active_device_kind(conn, &p)
                }
            }
        }
    };

    log::debug!(
        "[network] state={state} connectivity={connectivity} connected={connected} kind={kind:?}"
    );
    NetworkInfo { connected, kind }
}

fn push_to_ui(window: &slint::Weak<crate::StatusBarWindow>, info: &NetworkInfo) {
    if let Some(window) = window.upgrade() {
        let adapter = window.global::<crate::NetworkAdapter>();
        adapter.set_connected(info.connected);
        adapter.set_connection_type(info.kind.as_str().into());
        adapter.set_current_status(info.kind.as_str().into());
    }
}

fn push_coalesced(
    weak: &slint::Weak<crate::StatusBarWindow>,
    pending: &Arc<AtomicBool>,
    info: NetworkInfo,
) {
    if !pending.swap(true, Ordering::AcqRel) {
        let w = weak.clone();
        let p = pending.clone();
        let _ = slint::invoke_from_event_loop(move || {
            push_to_ui(&w, &info);
            p.store(false, Ordering::Release);
        });
    }
}

pub struct NetworkController;

impl NetworkController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let adapter = window.global::<crate::NetworkAdapter>();
        adapter.on_network_clicked(|| {
            log::info!("[network] clicked");
        });

        let pending = Arc::new(AtomicBool::new(false));
        let weak = window.as_weak();

        // Estado inicial pesimista hasta la primera lectura D-Bus (evita flash de "WiFi").
        let initial = NetworkInfo {
            connected: false,
            kind: ConnectionKind::None,
        };
        let w = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            push_to_ui(&w, &initial);
        });

        thread::spawn(move || {
            loop {
                let conn = match Connection::new_system() {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!(
                            "[network] cannot connect to D-Bus system bus: {e} — retry in 10s"
                        );
                        thread::sleep(RECONNECT_DELAY);
                        continue;
                    }
                };

                let rule = format!(
                    "type='signal',interface='org.freedesktop.DBus.Properties',\
                     member='PropertiesChanged',path='{NM_PATH}'"
                );
                if let Err(e) = conn.add_match_no_cb(&rule) {
                    log::error!("[network] failed to add D-Bus match rule: {e} — retry in 10s");
                    thread::sleep(RECONNECT_DELAY);
                    continue;
                }

                // Fuerza un connectivity check fresco al arrancar (best effort).
                {
                    let proxy = conn.with_proxy(NM_SERVICE, NM_PATH, DBUS_TIMEOUT);
                    let _: Result<(u32,), dbus::Error> =
                        proxy.method_call(NM_IFACE, "CheckConnectivity", ());
                }

                let mut last = read_network(&conn);
                log::info!(
                    "[network] initial — connected={} kind={:?}",
                    last.connected,
                    last.kind
                );
                push_coalesced(&weak, &pending, last);

                // Bucle de eventos: señales + relectura periódica (cubre check
                // de conectividad y reinicios de NM sin señales).
                loop {
                    match conn.channel().read_write(Some(RESYNC_INTERVAL)) {
                        Ok(_) => {}
                        Err(_) => {
                            log::warn!("[network] D-Bus disconnected — reconnecting");
                            break;
                        }
                    }
                    while conn.channel().pop_message().is_some() {}

                    let current = read_network(&conn);
                    if current != last {
                        if current.connected != last.connected {
                            log::info!(
                                "[network] {} (kind {:?} → {:?})",
                                if current.connected {
                                    "connected — internet available"
                                } else {
                                    "disconnected — no internet"
                                },
                                last.kind,
                                current.kind
                            );
                        } else {
                            log::info!("[network] bearer changed ({:?} → {:?})", last.kind, current.kind);
                        }
                        last = current;
                        push_coalesced(&weak, &pending, last);
                    }
                }

                thread::sleep(RECONNECT_DELAY);
            }
        });
    }
}
