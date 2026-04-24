use crate::core::data::TimeInfo;
use chrono::{Local, Datelike};

pub fn get_time_info() -> TimeInfo {
    // let now = Local::now();

    // let time = now.format("%-I:%M %p").to_string();
    // let date = now.format("%a %m/%d").to_string();

    // TimeInfo { time, date }

    let now = Local::now();

    let time = now.format("%-I:%M %p").to_string();

    let dias = ["Dom", "Lun", "Mar", "Mié", "Jue", "Vie", "Sáb"];
    let dia = dias[now.weekday().num_days_from_sunday() as usize];

    let date = format!("{} {:02}/{:02}", dia, now.month(), now.day());

    TimeInfo { time, date }
}