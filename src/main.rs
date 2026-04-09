slint::include_modules!();

use slint::Timer;
use std::time::Duration;

mod media;
mod system;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("SLINT_BACKEND", "winit-software");
    }

    env_logger::init();
    log::info!("Starting Status Bar");

    let window = StatusBar::new()?;

    let win = window.window();
    win.set_size(slint::PhysicalSize::new(1920, 32));
    win.set_position(slint::PhysicalPosition::new(0, 0));

    let window_weak = window.as_weak();
    let timer = Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(2),
        move || {
            if let Some(window) = window_weak.upgrade() {
                update_status(&window);
            }
        },
    );

    window.on_media_play_pause(move || {
        if let Ok(_) = media::play_pause() {
            log::info!("Media play/pause");
        }
    });

    window.on_media_next(move || {
        if let Ok(_) = media::next() {
            log::info!("Media next");
        }
    });

    window.on_media_previous(move || {
        if let Ok(_) = media::previous() {
            log::info!("Media previous");
        }
    });

    window.run()?;
    Ok(())
}

fn update_status(window: &StatusBar) {
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
    window.set_active_workspace(1);
}
