pub mod app_bar;
pub mod config;
pub mod position_monitor;
pub mod power;

pub use app_bar::get_window_position;
pub use config::{AppBarEdge, StatusBarConfig};
pub use power::BatteryMonitor;

pub fn init_statusbar(config: &StatusBarConfig, hwnd: isize) {
    println!(
        "[statusbar] init_statusbar: height={}, edge={:?}",
        config.height, config.edge
    );

    println!("[statusbar] Applying WS_EX_TOOLWINDOW + WS_EX_NOACTIVATE...");
    app_bar::apply_toolwindow_style(hwnd);

    println!("[statusbar] Hiding from taskbar with ITaskbarList3::DeleteTab...");
    app_bar::hide_from_taskbar(hwnd);

    app_bar::install_appbar_window_proc(hwnd);

    app_bar::force_window_position(hwnd, 0, 0, 1366, config.height);

    position_monitor::start_position_monitor(hwnd, config.height);
}

pub fn open_network_panel() {
    std::process::Command::new("explorer.exe")
        .arg("ms-availablenetworks:")
        .spawn()
        .expect("Failed to open network panel");
}

pub fn open_screen_clip() {
    std::process::Command::new("explorer.exe")
        .arg("ms-screenclip:")
        .spawn()
        .expect("Failed to open screen clip tool");
}

pub fn open_text_extractor() {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    unsafe {
        let inputs = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_LWIN,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_LSHIFT,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_T,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_T,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_LSHIFT,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_LWIN,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}
