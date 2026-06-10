pub mod types;

pub struct WorkspacesController;

impl WorkspacesController {
    pub fn connect(_window: &crate::StatusBarWindow) {
        log::info!("[workspaces] controller connected (placeholder)");
    }
}
