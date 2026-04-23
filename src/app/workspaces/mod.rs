pub mod komorebi;

pub use komorebi::start_komorebi_listener;

#[cfg(not(target_os = "windows"))]
pub fn start_komorebi_listener(_app_weak: slint::Weak<crate::StatusBarWindow>) {
    println!("[komorebi] not supported on this platform");
}