use serde::{Deserialize, Serialize};
use slint::Color;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub display: DisplayConfig,
    pub theme: ThemeConfig,
    pub widget: WidgetStylesConfig,
    pub widgets: WidgetsConfig,
    pub workspaces: WorkspacesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub height: i32,
    pub edge: String,
    pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub accent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WidgetStylesConfig {
    pub workspaces: Option<WorkspacesStyle>,
    pub music_icon: Option<MusicIconStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesStyle {
    pub bg_color: Option<String>,
    pub active_color: Option<String>,
    pub occupied_bg: Option<String>,
    pub text_active: Option<String>,
    pub text_occupied: Option<String>,
    pub text_free: Option<String>,
    pub border_radius: Option<i32>,
}

impl Default for WorkspacesStyle {
    fn default() -> Self {
        Self {
            bg_color: Some("#1e1f1bf6".to_string()),
            active_color: Some("#B4CCC1".to_string()),
            occupied_bg: Some("#303934".to_string()),
            text_active: Some("#303934".to_string()),
            text_occupied: Some("#B4C0B9".to_string()),
            text_free: Some("#868686".to_string()),
            border_radius: Some(14),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicIconStyle {
    pub artist_color: Option<String>,
    pub album_border_radius: Option<i32>,
}

impl Default for MusicIconStyle {
    fn default() -> Self {
        Self {
            artist_color: Some("#85948d".to_string()),
            album_border_radius: Some(4),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetsConfig {
    pub battery: bool,
    pub network: bool,
    pub bluetooth: bool,
    pub clock: bool,
    pub media: bool,
    pub workspaces: bool,
    pub screenshot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesConfig {
    pub total_workspaces: i32,
    pub format: Option<Vec<String>>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            height: 38,
            edge: "top".to_string(),
            width: 1366,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background: "#0808080f".to_string(),
            foreground: "#bac7bf".to_string(),
            accent: "#89b4fa".to_string(),
        }
    }
}

impl Default for WidgetsConfig {
    fn default() -> Self {
        Self {
            battery: true,
            network: true,
            bluetooth: true,
            clock: true,
            media: true,
            workspaces: true,
            screenshot: true,
        }
    }
}

impl Default for WorkspacesConfig {
    fn default() -> Self {
        Self {
            total_workspaces: 8,
            format: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            display: DisplayConfig::default(),
            theme: ThemeConfig::default(),
            widget: WidgetStylesConfig::default(),
            widgets: WidgetsConfig::default(),
            workspaces: WorkspacesConfig::default(),
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("barrita")
}

#[allow(dead_code)]
pub fn get_workspaces_style(config: &Config) -> WorkspacesStyle {
    config
        .widget
        .workspaces
        .clone()
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn get_music_icon_style(config: &Config) -> MusicIconStyle {
    config
        .widget
        .music_icon
        .clone()
        .unwrap_or_default()
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

pub fn load_or_create_config() -> Config {
    let config_path = get_config_path();

    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match toml::from_str(&content) {
                    Ok(config) => {
                        println!("[config] Loaded config from: {}", config_path.display());
                        return config;
                    }
                    Err(e) => {
                        println!("[config] Error parsing config: {}, using defaults", e);
                    }
                }
            }
            Err(e) => {
                println!("[config] Error reading config: {}, using defaults", e);
            }
        }
    } else {
        println!("[config] Config file not found, creating default at: {}", config_path.display());
    }

    let config = Config::default();
    save_config(&config);
    config
}

pub fn save_config(config: &Config) {
    let config_dir = get_config_dir();
    let config_path = get_config_path();

    if !config_dir.exists() {
        if let Err(e) = fs::create_dir_all(&config_dir) {
            println!("[config] Error creating config directory: {}", e);
            return;
        }
    }

    match toml::to_string_pretty(config) {
        Ok(content) => {
            if let Err(e) = fs::write(&config_path, content) {
                println!("[config] Error writing config: {}", e);
            } else {
                println!("[config] Config saved to: {}", config_path.display());
            }
        }
        Err(e) => {
            println!("[config] Error serializing config: {}", e);
        }
    }
}

#[allow(dead_code)]
pub fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
        Color::from_argb_u8(a, r, g, b)
    } else if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::from_argb_u8(255, r, g, b)
    } else {
        Color::from_rgb_u8(0, 0, 0)
    }
}