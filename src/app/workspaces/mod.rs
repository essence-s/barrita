pub mod data;
pub mod platform;

pub use data::WorkspaceInfo;

pub fn start_komorebi_listener<F>(callback: F)
where
    F: Fn(WorkspaceInfo) + Send + 'static,
{
    #[cfg(target_os = "windows")]
    platform::windows::komorebi::start_listener(callback);

    #[cfg(not(target_os = "windows"))]
    println!("[komorebi] not supported on this platform");
}