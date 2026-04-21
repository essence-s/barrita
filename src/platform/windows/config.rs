#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum AppBarEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[allow(dead_code)]
impl AppBarEdge {
    pub fn to_abe(&self) -> u32 {
        match self {
            AppBarEdge::Top => 1,
            AppBarEdge::Bottom => 3,
            AppBarEdge::Left => 0,
            AppBarEdge::Right => 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StatusBarConfig {
    pub height: i32,
    pub edge: AppBarEdge,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            height: 34,
            edge: AppBarEdge::Top,
        }
    }
}
