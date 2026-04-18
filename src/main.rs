slint::include_modules!();

use slint::{ComponentHandle, Timer, TimerMode};
use std::time::Duration;

use statusbar::{init_statusbar, AppBarEdge, StatusBarConfig};
mod statusbar;
use raw_window_handle::HasWindowHandle;

mod init;
mod popup;
mod status_updater;
mod media;
mod system;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init::configure_backend();

    env_logger::init();
    log::info!("Starting Barrita Status Bar");

    let app = StatusBarWindow::new()?;
    init::setup_window(&app);

    let app_weak = init::get_window_weak(&app);

    app.on_popup_toggle(move || {
        popup::toggle_popup(&app_weak);
    });

    app.on_media_play_pause(move || {
        let _ = media::play_pause();
    });

    app.on_media_next(move || {
        let _ = media::next();
    });

    app.on_media_previous(move || {
        let _ = media::previous();
    });

    // app.show()?;

    let app_weak = app.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(5), move || {
        if let Some(window) = app_weak.upgrade() {
            status_updater::update(&window);
        }
    });

    // slint::run_event_loop()?;
    let app_weak = app.as_weak();
    slint::invoke_from_event_loop(move || {
        let app = app_weak.unwrap();
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
                        height: 34,
                        edge: AppBarEdge::Top,
                    };
                    init_statusbar(&config, hwnd);

                    let rect = statusbar::get_window_position(hwnd);
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

    let _ = app.run();
    
    Ok(())
}