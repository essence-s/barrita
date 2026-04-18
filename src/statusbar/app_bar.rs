#![allow(dead_code)]

use super::config::AppBarEdge;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::UI::Shell::APPBARDATA;
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

pub const GWL_EXSTYLE: i32 = -20;
pub const WS_EX_NOACTIVATE: u32 = 0x08000000;
pub const WS_EX_TOPMOST: u32 = 0x00000008;

pub const SWP_NOSENDCHANGING: u32 = 0x0400;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_SHOWWINDOW: u32 = 0x0040;

pub const SPI_GETWORKAREA: u32 = 48;

pub const ABM_NEW: u32 = 0;
pub const ABM_REMOVE: u32 = 1;
pub const ABM_QUERYPOS: u32 = 2;
pub const ABM_SETPOS: u32 = 3;
pub const ABM_GETSTATE: u32 = 4;
pub const ABM_GETTASKBARPOS: u32 = 5;
pub const ABM_ACTIVATE: u32 = 6;
pub const ABM_GETAUTOHIDEBAR: u32 = 7;
pub const ABM_SETAUTOHIDEBAR: u32 = 8;
pub const ABM_WINDOWPOSCHANGED: u32 = 9;
pub const ABM_SETSTATE: u32 = 10;

pub const ABE_LEFT: u32 = 0;
pub const ABE_TOP: u32 = 1;
pub const ABE_RIGHT: u32 = 2;
pub const ABE_BOTTOM: u32 = 3;

pub const WM_USER: u32 = 0x0400;
pub const GWL_WNDPROC: i32 = -4;

pub const SW_HIDE: i32 = 0;
pub const SW_SHOW: i32 = 5;

pub const HWND_TOPMOST: isize = -1;
pub const HWND_NOTOPMOST: isize = -2;

pub fn get_systray_hwnd() -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    let class_name: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
    let title: Vec<u16> = vec![0; 1];

    unsafe {
        match FindWindowW(
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
        ) {
            Ok(hwnd) => {
                if hwnd.0.is_null() {
                    None
                } else {
                    Some(hwnd.0 as isize)
                }
            }
            Err(_) => None,
        }
    }
}

pub fn kill_systray_timer() {
    if let Some(hwnd) = get_systray_hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetWindowPos};

        unsafe {
            let flags = windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            let hwnd_notopmost =
                windows::Win32::Foundation::HWND(HWND_NOTOPMOST as *mut std::ffi::c_void);
            let _ = SetWindowPos(
                windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void),
                Some(hwnd_notopmost),
                0,
                0,
                0,
                0,
                flags,
            );
            let _ = KillTimer(
                Some(windows::Win32::Foundation::HWND(
                    hwnd as *mut std::ffi::c_void,
                )),
                1,
            );
        }
    }
}

pub fn restore_systray() {
    if let Some(hwnd) = get_systray_hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{SetTimer, SetWindowPos};

        unsafe {
            let flags = windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            let hwnd_topmost =
                windows::Win32::Foundation::HWND(HWND_TOPMOST as *mut std::ffi::c_void);
            let _ = SetWindowPos(
                windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void),
                Some(hwnd_topmost),
                0,
                0,
                0,
                0,
                flags,
            );
            let _ = SetTimer(
                Some(windows::Win32::Foundation::HWND(
                    hwnd as *mut std::ffi::c_void,
                )),
                1,
                100,
                None,
            );
        }
    }
}

pub fn get_window_position(hwnd: isize) -> RECT {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        let _ = GetWindowRect(
            windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void),
            &mut rect,
        );
    }
    rect
}

pub fn force_window_position(hwnd: isize, left: i32, top: i32, width: i32, height: i32) {
    use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;

    let flags =
        windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE | SWP_NOSIZE);
    let hwnd_topmost = windows::Win32::Foundation::HWND(HWND_TOPMOST as *mut std::ffi::c_void);

    unsafe {
        let _ = SetWindowPos(
            windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void),
            Some(hwnd_topmost),
            left,
            top,
            width,
            height,
            flags,
        );
    }
}

pub fn get_system_work_area() -> RECT {
    use windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;

    let mut work_area = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    unsafe {
        let _ = SystemParametersInfoW(
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_ACTION(SPI_GETWORKAREA),
            0,
            Some(&mut work_area as *mut RECT as *mut std::ffi::c_void),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }

    work_area
}

pub fn get_work_area() -> RECT {
    get_system_work_area()
}

pub fn debug_get_taskbar_state() {
    use windows::Win32::UI::Shell::SHAppBarMessage;

    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: windows::Win32::Foundation::HWND(std::ptr::null_mut()),
        uCallbackMessage: 0,
        uEdge: 0,
        rc: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        lParam: LPARAM(0),
    };

    unsafe {
        let result = SHAppBarMessage(ABM_GETTASKBARPOS, &mut data);
        println!("[app_bar] Taskbar state - result: {}", result != 0);
        if result != 0 {
            println!(
                "[app_bar] Taskbar position: left={}, top={}, right={}, bottom={}",
                data.rc.left, data.rc.top, data.rc.right, data.rc.bottom
            );
        }
    }
}

#[derive(Clone, Default)]
pub struct AppBar {
    registered: bool,
    hwnd: isize,
    edge: u32,
    height: i32,
}

impl AppBar {
    pub fn new() -> Self {
        AppBar {
            registered: false,
            hwnd: 0,
            edge: ABE_TOP,
            height: 34,
        }
    }

    pub fn register(&mut self, hwnd: isize, edge: AppBarEdge, height: i32) -> bool {
        use windows::Win32::UI::Shell::SHAppBarMessage;

        let edge_u32 = match edge {
            AppBarEdge::Top => ABE_TOP,
            AppBarEdge::Bottom => ABE_BOTTOM,
            AppBarEdge::Left => ABE_LEFT,
            AppBarEdge::Right => ABE_RIGHT,
        };

        println!(
            "[app_bar] register() called with hwnd={}, edge={}, height={}",
            hwnd, edge_u32, height
        );

        self.hwnd = hwnd;
        self.edge = edge_u32;
        self.height = height;

        let callback_msg = WM_USER + 100;

        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void),
            uCallbackMessage: callback_msg,
            uEdge: edge_u32,
            rc: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            lParam: LPARAM(0),
        };

        unsafe {
            let result = SHAppBarMessage(ABM_NEW, &mut data);
            println!("[app_bar] ABM_NEW result: {} (non-zero = success)", result);
            if result == 0 {
                return false;
            }
        }

        self.registered = true;
        println!("[app_bar] AppBar registered, calling set_position()");
        self.set_position()
    }

    pub fn set_position(&mut self) -> bool {
        use windows::Win32::UI::Shell::SHAppBarMessage;
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SetWindowPos};

        if !self.registered {
            return false;
        }

        let screen_width = unsafe {
            GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SYSTEM_METRICS_INDEX(78))
        };
        let screen_height = unsafe {
            GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SYSTEM_METRICS_INDEX(79))
        };

        println!("[app_bar] Screen: {}x{}", screen_width, screen_height);

        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: windows::Win32::Foundation::HWND(self.hwnd as *mut std::ffi::c_void),
            uCallbackMessage: 0,
            uEdge: self.edge,
            rc: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            lParam: LPARAM(0),
        };

        match self.edge {
            ABE_TOP => {
                data.rc.top = 0;
                data.rc.bottom = self.height;
                data.rc.left = 0;
                data.rc.right = screen_width;
                println!(
                    "[app_bar] TOP edge: rc =({}, {}, {}, {})",
                    data.rc.left, data.rc.top, data.rc.right, data.rc.bottom
                );
            }
            ABE_BOTTOM => {
                data.rc.top = screen_height - self.height;
                data.rc.bottom = screen_height;
                data.rc.left = 0;
                data.rc.right = screen_width;
                println!(
                    "[app_bar] BOTTOM edge: rc =({}, {}, {}, {})",
                    data.rc.left, data.rc.top, data.rc.right, data.rc.bottom
                );
            }
            ABE_LEFT => {
                data.rc.top = 0;
                data.rc.bottom = screen_height;
                data.rc.left = 0;
                data.rc.right = self.height;
            }
            ABE_RIGHT => {
                data.rc.top = 0;
                data.rc.bottom = screen_height;
                data.rc.left = screen_width - self.height;
                data.rc.right = screen_width;
            }
            _ => return false,
        }

        unsafe {
            println!("[app_bar] Calling ABM_QUERYPOS...");
            SHAppBarMessage(ABM_QUERYPOS, &mut data);
            println!(
                "[app_bar] After QUERYPOS: rc =({}, {}, {}, {})",
                data.rc.left, data.rc.top, data.rc.right, data.rc.bottom
            );

            println!("[app_bar] Calling ABM_SETPOS...");
            let setpos_result = SHAppBarMessage(ABM_SETPOS, &mut data);
            println!("[app_bar] ABM_SETPOS result: {}", setpos_result);
            println!(
                "[app_bar] After SETPOS: rc =({}, {}, {}, {})",
                data.rc.left, data.rc.top, data.rc.right, data.rc.bottom
            );

            println!("[app_bar] Calling SetWindowPos to position window...");
            let flags = windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(
                SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOMOVE,
            );
            let setpos_result = SetWindowPos(
                windows::Win32::Foundation::HWND(self.hwnd as *mut std::ffi::c_void),
                None,
                data.rc.left as i32,
                data.rc.top as i32,
                data.rc.right - data.rc.left,
                data.rc.bottom - data.rc.top,
                flags,
            );
            println!("[app_bar] SetWindowPos result: {:?}", setpos_result.is_ok());

            debug_get_taskbar_state();
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        let work_area_after = get_work_area();
        println!(
            "[app_bar] Work area after: top={}, bottom={}",
            work_area_after.top, work_area_after.bottom
        );

        true
    }

    pub fn set_window_style(&self, always_on_top: bool) {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW};

        unsafe {
            let current_ex_style = GetWindowLongPtrW(
                windows::Win32::Foundation::HWND(self.hwnd as *mut std::ffi::c_void),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX(GWL_EXSTYLE),
            );
            let mut updated_ex_style = current_ex_style as u32 | WS_EX_NOACTIVATE;

            if always_on_top {
                updated_ex_style |= WS_EX_TOPMOST;
                println!("[app_bar] Setting WS_EX_TOPMOST");
            }

            println!(
                "[app_bar] Setting window style: current={:#x}, new={:#x}",
                current_ex_style, updated_ex_style
            );

            let _ = SetWindowLongPtrW(
                windows::Win32::Foundation::HWND(self.hwnd as *mut std::ffi::c_void),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX(GWL_EXSTYLE),
                updated_ex_style as isize,
            );
        }
    }

    pub fn unregister(&mut self) {
        use windows::Win32::UI::Shell::SHAppBarMessage;

        if self.registered {
            let mut data = APPBARDATA {
                cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                hWnd: windows::Win32::Foundation::HWND(self.hwnd as *mut std::ffi::c_void),
                uCallbackMessage: 0,
                uEdge: self.edge,
                rc: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                lParam: LPARAM(0),
            };
            unsafe {
                let _ = SHAppBarMessage(ABM_REMOVE, &mut data);
            }
            self.registered = false;
        }
    }

    pub fn notify_pos_changed(&mut self) {
        use windows::Win32::UI::Shell::SHAppBarMessage;

        if self.registered {
            let mut data = APPBARDATA {
                cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                hWnd: windows::Win32::Foundation::HWND(self.hwnd as *mut std::ffi::c_void),
                uCallbackMessage: 0,
                uEdge: self.edge,
                rc: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                lParam: LPARAM(0),
            };
            unsafe {
                let _ = SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut data);
            }
        }
    }
}

impl Drop for AppBar {
    fn drop(&mut self) {
        self.unregister();
    }
}

static OLD_WNDPROC: std::sync::OnceLock<Option<isize>> = std::sync::OnceLock::new();
static APP_HWND: std::sync::OnceLock<isize> = std::sync::OnceLock::new();

unsafe extern "system" fn appbar_wndproc(
    hwnd: *mut std::ffi::c_void,
    msg: u32,
    wparam: isize,
    lparam: isize,
) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::{CallWindowProcW, ShowWindow};

    let appbar_callback_msg = WM_USER + 100;

    const ABN_POSCHANGED: i32 = 0;
    const ABN_STATECHANGE: i32 = 1;
    const ABN_FULLSCREENAPP: i32 = 2;
    const ABN_WINDOWARRANGE: i32 = 3;

    if msg == appbar_callback_msg {
        let notify_code = wparam as i32;
        match notify_code {
            ABN_POSCHANGED => {
                let mut bar = AppBar::new();
                bar.set_window_style(true);
                if bar.register(hwnd as isize, AppBarEdge::Top, 34) {
                    bar.notify_pos_changed();
                }
            }
            ABN_STATECHANGE => {}
            ABN_FULLSCREENAPP => {
                let is_fullscreen = lparam != 0;
                if is_fullscreen {
                    println!("[app_bar] fullscreen started, HIDING");
                    unsafe {
                        let _ = ShowWindow(
                            windows::Win32::Foundation::HWND(hwnd),
                            windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD(SW_HIDE),
                        );
                    }
                } else {
                    println!("[app_bar] fullscreen ended, SHOWING");
                    unsafe {
                        let _ = ShowWindow(
                            windows::Win32::Foundation::HWND(hwnd),
                            windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD(SW_SHOW),
                        );
                    }
                    force_window_position(hwnd as isize, 0, 0, 1366, 34);
                }
            }
            ABN_WINDOWARRANGE => {}
            _ => {}
        }
        return 0;
    }

    if let Some(&Some(old_proc)) = OLD_WNDPROC.get() {
        unsafe {
            let old: windows::Win32::UI::WindowsAndMessaging::WNDPROC =
                std::mem::transmute(old_proc);
            return CallWindowProcW(
                old,
                windows::Win32::Foundation::HWND(hwnd),
                msg,
                windows::Win32::Foundation::WPARAM(wparam as usize),
                windows::Win32::Foundation::LPARAM(lparam),
            )
            .0 as isize;
        }
    }
    0
}

pub fn install_appbar_window_proc(hwnd: isize) {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW};

    APP_HWND.get_or_init(|| hwnd);

    unsafe {
        let old_wndproc = GetWindowLongPtrW(
            windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void),
            windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX(GWL_WNDPROC),
        );
        OLD_WNDPROC.get_or_init(|| Some(old_wndproc));

        let new_wndproc = appbar_wndproc as *mut std::ffi::c_void;
        let _ = SetWindowLongPtrW(
            windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void),
            windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX(GWL_WNDPROC),
            new_wndproc as isize,
        );
    }
}
