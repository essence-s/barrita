use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BatteryStatusInfo {
    pub percentage: u8,
    pub is_charging: bool,
    pub is_low: bool,
}
