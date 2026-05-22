#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceInfo {
    pub active_workspace: i32,
    pub workspace_occupied: Vec<bool>,
}
