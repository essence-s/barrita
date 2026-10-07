#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Wifi,
    Ethernet,
    None,
}

impl ConnectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectionKind::Wifi => "wifi",
            ConnectionKind::Ethernet => "ethernet",
            ConnectionKind::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkInfo {
    pub connected: bool,
    pub kind: ConnectionKind,
}
