pub mod platform;

#[cfg(target_os = "windows")]
pub use platform::windows::open_action_center;
