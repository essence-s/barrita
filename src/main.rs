#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

use slint::{ComponentHandle, Timer, TimerMode};
use std::time::Duration;

mod core;
mod platform;
mod app;
mod popup;
mod status_updater;
mod config;
mod app_init;
mod gif_loader;
mod gif_animator;
mod ui;

const GIF_INTERVAL_MS: u64 = 400;

use config::load_or_create_config;
use ui::image::bytes_to_slint_image;
use app_init::init_app;

use platform::tray::init_tray;
use platform::windows::{get_window_position, init_statusbar, open_network_panel, open_screen_clip, open_text_extractor, open_action_center, AppBarEdge, StatusBarConfig, BatteryMonitor, start_media_listener, MediaUpdate};
use raw_window_handle::HasWindowHandle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("SLINT_BACKEND", "winit-femtovg");
        // std::env::set_var("SLINT_BACKEND", "winit-femtovg-wgpu");
        // std::env::set_var("SLINT_BACKEND", "winit-skia");
        // std::env::set_var("SLINT_BACKEND", "winit-skia-opengl");
    }

    env_logger::init();
    log::info!("Starting Barrita Status Bar");

    let cfg = load_or_create_config();

    let _tray = init_tray();

    let app = StatusBarWindow::new()?;
    init_app(&app, &cfg, 1920, 32);

    let animator = gif_animator::GifAnimator::new();
    animator.init(&app);
    let _gif_timer = animator.start_animation(app.as_weak(), GIF_INTERVAL_MS);

    let app_weak = app.as_weak();
    let app_weak_popup = app_weak.clone();
    app.on_popup_toggle(move || {
        popup::toggle_popup(&app_weak_popup);
    });

    app.on_network_clicked(move || {
        open_network_panel();
    });

    app.on_colorize_clicked(move || {
        open_screen_clip();
    });

    app.on_screenshot_clicked(move || {
        open_screen_clip();
    });

    app.on_article_clicked(move || {
        open_text_extractor();
    });

    app.on_bluetooth_clicked(move || {
        open_action_center();
    });

    let _monitor = BatteryMonitor::new(move |status| {
        let status = status.clone();
        let app_weak_clone = app_weak.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(window) = app_weak_clone.upgrade() {
                window.set_battery_percentage(format!("{}%", status.percentage).into());
                window.set_battery_charging(status.is_charging);
                window.set_battery_level(status.percentage as i32);
                window.set_battery_low(status.is_low);
            }
        }).ok();
    })?;

    // Media listener
    let app_weak_for_listener = app.as_weak();
    let _media_listener = start_media_listener(move |update| {
        let app_weak = app_weak_for_listener.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(window) = app_weak.upgrade() {
                match update {
                    MediaUpdate::PlaybackStatus(status) => {
                        println!("[main] >>> PlaybackStatus update: {}", status);
                        window.set_media_status(status.into());
                    }
                    MediaUpdate::MediaInfo { title, artist, status, has_player, thumbnail } => {
                        println!("[main] >>> MediaInfo update: {} - {} ({})", title, artist, status);
                        window.set_media_title(title.into());
                        window.set_media_artist(artist.into());
                        window.set_media_status(status.into());
                        window.set_media_has_player(has_player);

                        if let Some(bytes) = thumbnail {
                            if let Some(img) = bytes_to_slint_image(&bytes) {
                                println!("[main] >>> Thumbnail updated");
                                window.set_media_album_art(img);
                            }
                        }
                    }
                }
            }
        }).ok();
    })?;

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

                    let bar_edge = match cfg.display.edge.as_str() {
                        "bottom" => AppBarEdge::Bottom,
                        "left" => AppBarEdge::Left,
                        "right" => AppBarEdge::Right,
                        _ => AppBarEdge::Top,
                    };

                    let config = StatusBarConfig {
                        height: cfg.display.height,
                        edge: bar_edge,
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
        let app_weak = app.as_weak();
        app::workspaces::start_komorebi_listener(move |info| {
            println!("[main] Workspace changed: active={}, occupied={:?}", 
                info.active_workspace, info.workspace_occupied);
            let app_weak_clone = app_weak.clone();
            if let Err(e) = slint::invoke_from_event_loop(move || {
                if let Some(window) = app_weak_clone.upgrade() {
                    let occupied: Vec<bool> = info.workspace_occupied.clone();
                    let model = slint::VecModel::from(occupied);
                    window.set_active_workspace(info.active_workspace);
                    window.set_workspace_occupied(slint::ModelRc::new(model));
                }
            }) {
                eprintln!("[komorebi] Failed to update UI: {}", e);
            }
        });
    }

    let _ = app.run();
    
    Ok(())
}