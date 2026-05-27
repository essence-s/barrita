pub mod data;
pub mod provider;
pub mod platform;

pub use data::NetworkInfo;
pub use provider::get_network_info;

#[cfg(target_os = "windows")]
pub use platform::windows::open_network_panel;