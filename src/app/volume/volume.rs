use crate::core::data::VolumeInfo;

pub fn get_volume_info() -> Option<VolumeInfo> {
    #[cfg(target_os = "windows")]
    {
        use volumecontrol::AudioDevice;

        let device = AudioDevice::from_default().ok()?;
        let volume = device.get_vol().ok()?;
        let muted = volume == 0;

        let icon = match volume {
            0 => "volume_off",
            1..=33 => "volume_mute",
            34..=66 => "volume_down",
            _ => "volume_up",
        };

        return Some(VolumeInfo {
            volume,
            muted,
            icon: icon.to_string(),
        });
    }

    #[cfg(not(target_os = "windows"))]
    {
        Some(VolumeInfo {
            volume: 50,
            muted: false,
            icon: "volume_up".to_string(),
        })
    }
}

#[allow(dead_code)]
pub fn increase_volume() -> Result<u8, String> {
    #[cfg(target_os = "windows")]
    {
        use volumecontrol::AudioDevice;

        let device = AudioDevice::from_default().map_err(|e| format!("{}", e))?;
        let current = device.get_vol().map_err(|e| format!("{}", e))?;
        let new_volume = (current + 10).min(100);
        device.set_vol(new_volume).map_err(|e| format!("{}", e))?;
        Ok(new_volume)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(60)
    }
}

#[allow(dead_code)]
pub fn decrease_volume() -> Result<u8, String> {
    #[cfg(target_os = "windows")]
    {
        use volumecontrol::AudioDevice;

        let device = AudioDevice::from_default().map_err(|e| format!("{}", e))?;
        let current = device.get_vol().map_err(|e| format!("{}", e))?;
        let new_volume = current.saturating_sub(10);
        device.set_vol(new_volume).map_err(|e| format!("{}", e))?;
        Ok(new_volume)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(40)
    }
}

#[allow(dead_code)]
pub fn set_volume(volume: u8) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use volumecontrol::AudioDevice;

        let device = AudioDevice::from_default().map_err(|e| format!("{}", e))?;
        device.set_vol(volume).map_err(|e| format!("{}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = volume;
        Ok(())
    }
}

#[allow(dead_code)]
pub fn toggle_mute() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use volumecontrol::AudioDevice;

        let device = AudioDevice::from_default().map_err(|e| format!("{}", e))?;
        let current = device.get_vol().unwrap_or(0);

        if current > 0 {
            device.set_vol(0).map_err(|e| format!("{}", e))?;
            Ok(true)
        } else {
            device.set_vol(50).map_err(|e| format!("{}", e))?;
            Ok(false)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}