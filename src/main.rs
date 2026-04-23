slint::include_modules!();

use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Timer, TimerMode};
use std::time::Duration;

mod core;
mod platform;
mod app;
mod popup;
mod status_updater;

use platform::windows::{get_window_position, init_statusbar, AppBarEdge, StatusBarConfig};
use raw_window_handle::HasWindowHandle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("SLINT_BACKEND", "winit-femtovg");
    }

    env_logger::init();
    log::info!("Starting Barrita Status Bar");

    let app = StatusBarWindow::new()?;
    app.window().set_size(PhysicalSize::new(1920, 32));
    app.window().set_position(PhysicalPosition::new(0, 0));

    let app_weak = app.as_weak();
    app.on_popup_toggle(move || {
        popup::toggle_popup(&app_weak);
    });

    app.on_media_play_pause(move || {
        let _ = app::media::play_pause();
    });

    app.on_media_next(move || {
        let _ = app::media::next();
    });

    app.on_media_previous(move || {
        let _ = app::media::previous();
    });

    let app_weak = app.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(5), move || {
        if let Some(window) = app_weak.upgrade() {
            status_updater::update(&window);
        }
    });

    let init_app_weak = app.as_weak();
    slint::invoke_from_event_loop(move || {
        let app = init_app_weak.upgrade().unwrap();
        let window = app.window();
        let handle = window.window_handle();

        match handle.window_handle() {
            Ok(win_handle) => match win_handle.as_ref() {
                raw_window_handle::RawWindowHandle::Win32(win32_handle) => {
                    let hwnd = win32_handle.hwnd.get() as isize;
                    println!("[main] HWND obtained: {}", hwnd);

                    println!("[main] Showing window first...");
                    window.show().unwrap();

                    let config = StatusBarConfig {
                        height: 38,
                        edge: AppBarEdge::Top,
                    };
                    init_statusbar(&config, hwnd);

                    let rect = get_window_position(hwnd);
                    println!("[main] Window rect: left={}, top={}", rect.left, rect.top);
                }
                _ => {
                    println!("[main] ERROR: Not a Win32 handle");
                }
            },
            Err(e) => {
                println!("[main] ERROR getting window handle: {:?}", e);
            }
        }
    })
    .unwrap();

    #[cfg(target_os = "windows")]
    {
        let komorebi_app_weak = app.as_weak();
        app::workspaces::start_komorebi_listener(komorebi_app_weak);
    }

    let _ = app.run();
    
    Ok(())
}