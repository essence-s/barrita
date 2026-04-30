use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::ffi::c_void;
use windows::{
    core::GUID,
    Win32::{
        System::Power::{
            PowerSettingRegisterNotification,
            UnregisterPowerSettingNotification,
            DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS,
            HPOWERNOTIFY,
        },
        UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK,
        Foundation::{HANDLE, WIN32_ERROR},
    },
};

const GUID_ACDC_POWER_SOURCE: GUID = GUID::from_u128(0x5D3E9A59_E9D5_4B00_A6BD_FF34FF516548);
const GUID_BATTERY_PERCENTAGE_REMAINING: GUID = GUID::from_u128(0xA7AD8041_B45A_4CAE_87A3_EECBB468A9E1);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerSource {
    Ac = 0,
    Dc = 1,
    Hot = 2,
}

#[derive(Debug, Clone)]
pub enum PowerEvent {
    PowerSource(PowerSource),
    BatteryPercentage(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct BatteryStatusInfo {
    pub percentage: u8,
    pub is_charging: bool,
    pub is_low: bool,
}

pub struct BatteryMonitor {
    handles: Vec<HPOWERNOTIFY>,
    _receiver: Arc<Mutex<Option<mpsc::Receiver<PowerEvent>>>>,
}

#[repr(C)]
struct POWERBROADCAST_SETTING {
    PowerSetting: GUID,
    DataLength: u32,
    Data: u32,
}

static g_tx: std::sync::OnceLock<Box<mpsc::Sender<PowerEvent>>> = std::sync::OnceLock::new();

impl BatteryMonitor {
    pub fn new<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: 'static + Send + Fn(BatteryStatusInfo) + Clone,
    {
        let (tx, rx) = mpsc::channel::<PowerEvent>();
        let _ = g_tx.set(Box::new(tx));
        
        let callback_clone = callback.clone();
        
        unsafe extern "system" fn power_callback(
            _context: *const c_void,
            _event: u32,
            setting: *const c_void,
        ) -> u32 {
            if setting.is_null() {
                return 0;
            }
            
            let ps = &*(setting as *const POWERBROADCAST_SETTING);
            
            println!("[Power] Event: GUID={:?}, DataLength={}, Data={}", 
                ps.PowerSetting, ps.DataLength, ps.Data);
            
            if ps.PowerSetting == GUID_ACDC_POWER_SOURCE {
                let power_source = match ps.Data {
                    0 => PowerSource::Ac,
                    1 => PowerSource::Dc,
                    2 => PowerSource::Hot,
                    _ => PowerSource::Dc,
                };
                println!("[Power] PowerSource: {:?}", power_source);
                if let Some(tx) = g_tx.get() {
                    let _ = tx.send(PowerEvent::PowerSource(power_source));
                }
            } else if ps.PowerSetting == GUID_BATTERY_PERCENTAGE_REMAINING {
                let percentage = (ps.Data & 0xFF) as u8;
                println!("[Power] BatteryPercentage: {}%", percentage);
                if let Some(tx) = g_tx.get() {
                    let _ = tx.send(PowerEvent::BatteryPercentage(percentage));
                }
            }
            
            0
        }

        let mut handles: Vec<HPOWERNOTIFY> = Vec::new();
        let mut notify_params = DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(power_callback),
            Context: std::ptr::null_mut(),
        };
        
        let recipient = HANDLE(&mut notify_params as *mut _ as *mut c_void);
        
        println!("[Power] Registering GUID_ACDC_POWER_SOURCE...");
        let mut registration_handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            PowerSettingRegisterNotification(
                &GUID_ACDC_POWER_SOURCE,
                DEVICE_NOTIFY_CALLBACK,
                recipient,
                &mut registration_handle,
            )
        };
        if result == WIN32_ERROR(0) {
            println!("[Power] Registered GUID_ACDC_POWER_SOURCE: {:?}", registration_handle);
            handles.push(HPOWERNOTIFY(registration_handle as isize));
        } else {
            println!("[Power] Failed to register GUID_ACDC_POWER_SOURCE: error={:?}", result);
        }
        
        println!("[Power] Registering GUID_BATTERY_PERCENTAGE_REMAINING...");
        let mut registration_handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            PowerSettingRegisterNotification(
                &GUID_BATTERY_PERCENTAGE_REMAINING,
                DEVICE_NOTIFY_CALLBACK,
                recipient,
                &mut registration_handle,
            )
        };
        if result == WIN32_ERROR(0) {
            println!("[Power] Registered GUID_BATTERY_PERCENTAGE_REMAINING: {:?}", registration_handle);
            handles.push(HPOWERNOTIFY(registration_handle as isize));
        } else {
            println!("[Power] Failed to register GUID_BATTERY_PERCENTAGE_REMAINING: error={:?}", result);
        }
        
        let receiver = Arc::new(Mutex::new(Some(rx)));
        
        let receiver_clone = receiver.clone();
        thread::spawn(move || {
            let rx = receiver_clone.lock().unwrap().take().unwrap();
            let callback = callback_clone;
            
            let mut current_power_source = None;
            let mut current_percentage: u8 = 100;
            
            loop {
                match rx.recv() {
                    Ok(event) => match event {
                        PowerEvent::PowerSource(source) => {
                            current_power_source = Some(source);
                            println!("[Power] Thread received PowerSource: {:?}", source);
                        }
                        PowerEvent::BatteryPercentage(pct) => {
                            current_percentage = pct;
                            println!("[Power] Thread received BatteryPercentage: {}%", pct);
                        }
                    },
                    Err(e) => {
                        println!("[Power] Receiver error: {:?}", e);
                        break;
                    }
                }
                
                let is_charging = current_power_source == Some(PowerSource::Ac);
                let is_low = current_percentage <= 15 && !is_charging;
                
                let status = BatteryStatusInfo {
                    percentage: current_percentage,
                    is_charging,
                    is_low,
                };
                
                println!("[Power] Notifying: percentage={}%, is_charging={}, is_low={}", 
                    status.percentage, status.is_charging, status.is_low);
                
                callback(status);
            }
        });

        Ok(BatteryMonitor {
            handles,
            _receiver: receiver,
        })
    }
}

impl Drop for BatteryMonitor {
    fn drop(&mut self) {
        println!("[Power] Dropping - unregistering notifications...");
        for handle in &self.handles {
            unsafe {
                let _ = UnregisterPowerSettingNotification(*handle);
            }
        }
    }
}