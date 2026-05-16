pub mod komorebi;

pub use komorebi::start_komorebi_listener;

#[cfg(not(target_os = "windows"))]
pub fn start_komorebi_listener<F>(_callback: F)
where
    F: Fn(crate::app::workspaces::komorebi::WorkspaceInfo) + Send + 'static,
{
    println!("[komorebi] not supported on this platform");
}