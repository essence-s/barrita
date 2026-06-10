#[allow(dead_code)]
pub struct WorkspaceInfo {
    pub active: i32,
    pub occupied: Vec<bool>,
    pub total: i32,
    pub labels: Vec<String>,
}
