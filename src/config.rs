use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub display: DisplayConfig,
    pub theme: ThemeConfig,
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