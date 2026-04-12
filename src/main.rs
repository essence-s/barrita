slint::include_modules!();

use slint::{ComponentHandle, Image, Timer, Weak};
use std::sync::Mutex;
use std::time::Duration;

mod media;
mod system;

static LAST_ALBUM_BYTES: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static THUMBNAIL_COUNTER: Mutex<u32> = Mutex::new(0);
static POPUP_VISIBLE: Mutex<bool> = Mutex::new(false);
static POPUP_WEAK: Mutex<Option<Weak<MusicPopupWindow>>> = Mutex::new(None);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    unsafe {
        // std::env::set_var("SLINT_BACKEND", "winit-software");
        std::env::set_var("SLINT_BACKEND", "winit-skia");
        // std::env::set_var("SLINT_BACKEND", "winit-femtovg");
    }

    env_logger::init();
    log::info!("Starting Barrita Status Bar");

    let app = StatusBarWindow::new().unwrap();

    let win = app.window();
    win.set_size(slint::PhysicalSize::new(1920, 32));
    win.set_position(slint::PhysicalPosition::new(0, 0));

    let app_weak = app.as_weak();

    app.on_popup_toggle(move || {
        let is_visible = *POPUP_VISIBLE.lock().unwrap();
        println!("Popup toggle: current visible={}", is_visible);

        if is_visible {
            println!("Hiding popup");
            let mut popup_weak_guard = POPUP_WEAK.lock().unwrap();
            if let Some(weak) = popup_weak_guard.take() {
                if let Some(popup) = weak.upgrade() {
                    popup.hide().unwrap();
                }
            }
            *POPUP_VISIBLE.lock().unwrap() = false;
        } else {
            println!("Creating and showing new popup");
            if let Some(app) = app_weak.upgrade() {
                let pos = app.window().position();
                let size = app.window().size();
                let popup_x = pos.x;
                let popup_y = pos.y + size.height as i32;

                let popup = MusicPopupWindow::new().unwrap();
                popup.window().set_position(slint::WindowPosition::Physical(
                    slint::PhysicalPosition::new(popup_x, popup_y),
                ));
                popup.show().unwrap();

                *POPUP_WEAK.lock().unwrap() = Some(popup.as_weak());
                *POPUP_VISIBLE.lock().unwrap() = true;
                println!("Popup shown successfully");
            }
        }
    });

    app.on_media_play_pause(move || {
        if let Ok(_) = media::play_pause() {
            log::info!("Media play/pause");
        }
    });

    app.on_media_next(move || {
        if let Ok(_) = media::next() {
            log::info!("Media next");
        }
    });

    app.on_media_previous(move || {
        if let Ok(_) = media::previous() {
            log::info!("Media previous");
        }
    });

    app.show().unwrap();

    let app_weak = app.as_weak();
    let timer = Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(5),
        move || {
            if let Some(window) = app_weak.upgrade() {
                update_status(&window);
            }
        },
    );

    slint::run_event_loop().unwrap();
    Ok(())
}

fn update_status(window: &StatusBarWindow) {
    let mut data = system::StatusBarData::new();
    data.refresh();

    window.set_time(data.time.into());
    window.set_date(data.date.into());
    window.set_battery_percentage(data.battery_percentage.into());
    window.set_battery_charging(data.battery_charging);
    window.set_battery_icon(data.battery_icon.into());
    window.set_network_status(data.network_status.into());
    window.set_network_connected(data.network_connected);
    window.set_network_icon(data.network_icon.into());
    window.set_media_status(data.media_status.into());
    window.set_media_has_player(data.media_has_player);
    window.set_media_title(data.media_title.into());
    window.set_media_artist(data.media_artist.into());
    window.set_media_progress(data.media_progress);
    window.set_media_progress_time(data.media_progress_time.into());
    window.set_media_total_time(data.media_total_time.into());
    window.set_active_workspace(1);

    let mut counter_guard = THUMBNAIL_COUNTER.lock().unwrap();
    *counter_guard += 1;
    let should_update_thumbnail = *counter_guard % 2 == 0;
    drop(counter_guard);

    if should_update_thumbnail && !data.media_album_art.is_empty() {
        let mut last_bytes = LAST_ALBUM_BYTES.lock().unwrap();

        if data.media_album_art.len() > 100 && data.media_album_art != *last_bytes {
            log::info!("Processing thumbnail: {} bytes", data.media_album_art.len());

            match image::load_from_memory(&data.media_album_art) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (width, height) = rgba.dimensions();

                    if width > 0 && height > 0 && width < 500 && height < 500 {
                        let raw = rgba.into_raw();
                        let img_buffer =
                            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                                &raw, width, height,
                            );
                        let slint_img = Image::from_rgba8_premultiplied(img_buffer);
                        window.set_media_album_art(slint_img);
                        log::info!("Thumbnail loaded: {}x{}", width, height);
                        *last_bytes = data.media_album_art;
                    } else {
                        log::warn!("Thumbnail too large: {}x{}", width, height);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to decode thumbnail: {:?}", e);
                }
            }
        }
    }
}
