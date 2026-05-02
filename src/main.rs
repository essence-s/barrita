#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Timer, TimerMode, VecModel, ModelRc, SharedString};
use std::time::Duration;

mod core;
mod platform;
mod app;
mod popup;
mod status_updater;
mod config;

use config::{get_workspaces_style, get_music_icon_style, workspaces_style_to_slint, music_icon_style_to_slint, load_or_create_config};

use platform::tray::init_tray;
use platform::windows::{get_window_position, init_statusbar, open_network_panel, open_screen_clip, open_text_extractor, AppBarEdge, StatusBarConfig, BatteryMonitor};
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
    app.window().set_size(PhysicalSize::new(1920, 32));
    app.window().set_position(PhysicalPosition::new(0, 0));

    app.set_workspace_total(cfg.workspaces.total_workspaces);
    if let Some(ref format) = cfg.workspaces.format {
        let shared_format: Vec<SharedString> = format.into_iter().map(|s| s.into()).collect();
        let model = VecModel::from(shared_format);
        app.set_workspace_format(ModelRc::new(model));
    }

    let workspaces_style = get_workspaces_style(&cfg);
    let (bg, active, occupied, text_active, text_occupied, text_free, border) = workspaces_style_to_slint(&workspaces_style);
    app.set_workspaces_bg_color(bg);
    app.set_workspaces_active_color(active);
    app.set_workspaces_occupied_bg(occupied);
    app.set_workspaces_text_active(text_active);
    app.set_workspaces_text_occupied(text_occupied);
    app.set_workspaces_text_free(text_free);
    app.set_workspaces_border_radius(border);

    let music_icon_style = get_music_icon_style(&cfg);
    let (artist_color, album_border) = music_icon_style_to_slint(&music_icon_style);
    app.set_music_artist_color(artist_color);
    app.set_music_album_border_radius(album_border);

    let app_weak = app.as_weak();
    let app_weak_popup = app_weak.clone();
    app.on_popup_toggle(move || {
        popup::toggle_popup(&app_weak_popup);
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
        let komorebi_app_weak = app.as_weak();
        app::workspaces::start_komorebi_listener(komorebi_app_weak);
    }

    let _ = app.run();
    
    Ok(())
}