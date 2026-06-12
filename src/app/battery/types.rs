#[derive(Clone, PartialEq)]
pub struct BatteryStatusInfo {
    pub percentage: String,
    pub level: i32,
    pub charging: bool,
    pub low: bool,
}
