use crate::system::NetworkInfo;
use sysinfo::Networks;

pub fn get_network_info() -> NetworkInfo {
    let networks = Networks::new_with_refreshed_list();

    let mut connected = false;
    let mut status = "Disconnected".to_string();

    for (name, data) in &networks {
        let received = data.total_received();
        let transmitted = data.total_transmitted();

        if received > 0 || transmitted > 0 {
            connected = true;
            status = name.clone();
            break;
        }
    }

    let icon = if connected { "wifi" } else { "signal_wifi_off" };

    NetworkInfo {
        status,
        connected,
        icon: icon.to_string(),
    }
}
