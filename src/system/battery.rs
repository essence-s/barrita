use crate::system::BatteryInfo;
use battery::Manager;

pub fn get_battery_info() -> Option<BatteryInfo> {
    let manager = Manager::new().ok()?;

    for bat in manager.batteries().ok()?.flatten() {
        let percentage = bat.state_of_charge().value.round() as u8;
        let is_charging = matches!(bat.state(), battery::State::Charging);

        let mut icon = match percentage {
            0..=15 => "battery_1_bar",
            16..=35 => "battery_2_bar",
            36..=55 => "battery_3_bar",
            56..=75 => "battery_4_bar",
            76..=90 => "battery_5_bar",
            _ => "battery_6_bar",
        };

        if is_charging {
            icon = "battery_charging";
        }

        return Some(BatteryInfo {
            percentage,
            is_charging,
            icon: icon.to_string(),
        });
    }

    None
}
