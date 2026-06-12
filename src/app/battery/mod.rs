pub mod types;

use dbus::blocking::Connection;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use types::BatteryStatusInfo;

const UPOWER_DEVICE: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const UPOWER_SERVICE: &str = "org.freedesktop.UPower";
const UPOWER_IFACE: &str = "org.freedesktop.UPower.Device";

fn find_line_power(conn: &Connection) -> Option<String> {
    let candidates = [
        "/org/freedesktop/UPower/devices/line_power_AC",
        "/org/freedesktop/UPower/devices/line_power_ADP0",
        "/org/freedesktop/UPower/devices/line_power_AC0",
        "/org/freedesktop/UPower/devices/line_power_ADP1",
    ];
    for path in &candidates {
        let proxy = conn.with_proxy(UPOWER_SERVICE, *path, Duration::from_secs(3));
        if let Ok(kind) = proxy.get::<u32>(UPOWER_IFACE, "Type") {
            if kind == 1 {
                log::info!("[battery] found line_power device at {path}");
                return Some(path.to_string());
            }
        }
    }
    None
}

fn read_ac_online(conn: &Connection, ac_path: &str) -> bool {
    let proxy = conn.with_proxy(UPOWER_SERVICE, ac_path, Duration::from_secs(5));
    proxy.get::<bool>(UPOWER_IFACE, "Online").unwrap_or(false)
}

fn read_battery(conn: &Connection, ac_path: Option<&str>) -> Option<BatteryStatusInfo> {
    let proxy = conn.with_proxy(UPOWER_SERVICE, UPOWER_DEVICE, Duration::from_secs(5));

    let percentage = proxy.get::<f64>(UPOWER_IFACE, "Percentage").ok()?;
    let state = proxy.get::<u32>(UPOWER_IFACE, "State").ok()?;

    let level = percentage as i32;
    let from_state = state == 1 || state == 4;
    let charging = ac_path
        .map(|p| read_ac_online(conn, p))
        .unwrap_or(from_state);
    let low = !charging && level <= 20;

    Some(BatteryStatusInfo {
        percentage: format!("{}%", level),
        level: level.clamp(0, 100),
        charging,
        low,
    })
}

fn push_to_ui(window: &slint::Weak<crate::StatusBarWindow>, info: &BatteryStatusInfo) {
    if let Some(window) = window.upgrade() {
        let adapter = window.global::<crate::BatteryAdapter>();
        adapter.set_current_percentage(info.percentage.clone().into());
        adapter.set_level(info.level);
        adapter.set_charging(info.charging);
        adapter.set_low(info.low);
    }
}

pub struct BatteryController;

impl BatteryController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let pending = Arc::new(AtomicBool::new(false));
        let weak = window.as_weak();

        thread::spawn(move || {
            let conn = match Connection::new_system() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("[battery] cannot connect to D-Bus system bus: {e}");
                    return;
                }
            };

            let ac_path = find_line_power(&conn);

            let mut paths = vec![UPOWER_DEVICE.to_string()];
            if let Some(ref p) = ac_path {
                paths.push(p.clone());
            }
            for path in &paths {
                let rule = format!(
                    "type='signal',interface='org.freedesktop.DBus.Properties',\
                     member='PropertiesChanged',path='{path}'"
                );
                if let Err(e) = conn.add_match_no_cb(&rule) {
                    log::error!("[battery] failed to add D-Bus match rule: {e}");
                    return;
                }
            }

            let mut last = match read_battery(&conn, ac_path.as_deref()) {
                Some(info) => info,
                None => {
                    log::warn!("[battery] could not read initial state");
                    BatteryStatusInfo {
                        percentage: "0%".into(),
                        level: 0,
                        charging: false,
                        low: false,
                    }
                }
            };

            let w = weak.clone();
            let info = last.clone();
            let _ = slint::invoke_from_event_loop(move || {
                push_to_ui(&w, &info);
            });

            log::info!("[battery] listening for UPower D-Bus events");

            loop {
                conn.channel()
                    .read_write(None)
                    .expect("[battery] D-Bus disconnected");

                while let Some(_msg) = conn.channel().pop_message() {
                    if let Some(current) = read_battery(&conn, ac_path.as_deref()) {
                        if current != last {
                            if current.charging != last.charging {
                                if current.charging {
                                    log::info!(
                                        "[battery] charger connected — {}% and charging",
                                        current.level
                                    );
                                } else {
                                    log::info!(
                                        "[battery] charger disconnected — {}% and discharging",
                                        current.level
                                    );
                                }
                            } else {
                                log::info!(
                                    "[battery] {} {}%",
                                    if current.charging {
                                        "charging"
                                    } else {
                                        "discharging"
                                    },
                                    current.level
                                );
                            }

                            last = current;
                            let w = weak.clone();
                            let p = pending.clone();
                            if !p.swap(true, Ordering::AcqRel) {
                                let info = last.clone();
                                let _ = slint::invoke_from_event_loop(move || {
                                    push_to_ui(&w, &info);
                                    p.store(false, Ordering::Release);
                                });
                            }
                        }
                    }
                }
            }
        });
    }
}
