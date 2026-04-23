pub mod komorebi;

#[cfg(target_os = "windows")]
pub use komorebi::start_komorebi_listener;

#[cfg(not(target_os = "windows"))]
pub fn start_komorebi_listener() {
    println!("[komorebi] not supported on this platform");
}