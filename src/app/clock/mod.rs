use slint::{ComponentHandle, Timer, TimerMode};
use std::time::Duration;
use chrono::{Local, Datelike};

pub mod types;

use types::TimeInfo;

fn get_time_info() -> TimeInfo {
    let now = Local::now();
    let time = now.format("%-I:%M %p").to_string();

    let dias = ["Dom", "Lun", "Mar", "Mié", "Jue", "Vie", "Sáb"];
    let dia = dias[now.weekday().num_days_from_sunday() as usize];
    let date = format!("{} {:02}/{:02}", dia, now.day(), now.month());

    TimeInfo { time, date }
}

pub struct ClockController;

impl ClockController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let window_weak = window.as_weak();

        let update = move || {
            let info = get_time_info();
            if let Some(window) = window_weak.upgrade() {
                let adapter = window.global::<crate::ClockAdapter>();
                adapter.set_current_time(info.time.into());
                adapter.set_current_date(info.date.into());
            }
        };

        update();

        let timer = Box::new(Timer::default());
        timer.start(TimerMode::Repeated, Duration::from_secs(1), update);
        Box::leak(timer);
    }
}
