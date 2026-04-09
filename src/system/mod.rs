pub mod battery;
pub mod network;
pub mod processes;
pub mod time;
pub mod volume;

pub use battery::get_battery_info;
pub use network::get_network_info;
pub use processes::get_top_process;
pub use time::get_time_info;
pub use volume::{decrease_volume, get_volume_info, increase_volume, set_volume, toggle_mute};

use crate::media::get_media_info;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryInfo {
    pub percentage: u8,
    pub is_charging: bool,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    pub status: String,
    pub connected: bool,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VolumeInfo {
    pub volume: u8,
    pub muted: bool,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeInfo {
    pub time: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessInfo {
    pub top_process: String,
    pub cpu_usage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatusBarData {
    pub time: String,
    pub date: String,
    pub battery_percentage: String,
    pub battery_charging: bool,
    pub battery_icon: String,
    pub network_status: String,
    pub network_connected: bool,
    pub network_icon: String,
    pub volume: i32,
    pub volume_muted: bool,
    pub volume_icon: String,
    pub top_process: String,
    pub media_title: String,
    pub media_artist: String,
    pub media_status: String,
    pub media_has_player: bool,
}

impl StatusBarData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh(&mut self) {
        let info = get_time_info();
        self.time = info.time;
        self.date = info.date;

        if let Some(info) = get_battery_info() {
            self.battery_percentage = format!("{}%", info.percentage);
            self.battery_charging = info.is_charging;
            self.battery_icon = if info.is_charging {
                "⚡".to_string()
            } else {
                info.icon
            };
        } else {
            self.battery_percentage = "N/A".to_string();
            self.battery_icon = "🔋".to_string();
        }

        let net_info = get_network_info();
        self.network_status = net_info.status;
        self.network_connected = net_info.connected;
        self.network_icon = net_info.icon;

        if let Some(info) = get_volume_info() {
            self.volume = info.volume as i32;
            self.volume_muted = info.muted;
            self.volume_icon = info.icon;
        } else {
            self.volume = 0;
            self.volume_icon = "🔊".to_string();
        }

        let proc_info = get_top_process();
        self.top_process = proc_info.top_process;

        if let Some(info) = get_media_info() {
            self.media_title = info.title;
            self.media_artist = info.artist;
            self.media_status = info.status;
            self.media_has_player = info.has_player;
        } else {
            self.media_title = String::new();
            self.media_artist = String::new();
            self.media_status = "stopped".to_string();
            self.media_has_player = false;
        }
    }
}
