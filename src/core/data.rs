use serde::{Deserialize, Serialize};
use crate::app::network::get_network_info;
use crate::app::volume::get_volume_info;
use crate::app::clock::get_time_info;

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
    pub media_title: String,
    pub media_artist: String,
    pub media_status: String,
    pub media_has_player: bool,
    pub media_album_art: Vec<u8>,
    pub media_progress: f32,
    pub media_progress_time: String,
    pub media_total_time: String,
}

impl StatusBarData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh(&mut self) {
        let info = get_time_info();
        self.time = info.time;
        self.date = info.date;

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
    }
}