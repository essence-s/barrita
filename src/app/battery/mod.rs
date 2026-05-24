pub mod data;
pub mod platform;

pub use data::BatteryStatusInfo;

#[cfg(target_os = "windows")]
pub use platform::windows::power::BatteryMonitor;
