use crate::system::TimeInfo;
use chrono::Local;

pub fn get_time_info() -> TimeInfo {
    let now = Local::now();

    let time = now.format("%-I:%M %p").to_string();
    let date = now.format("%a %b %-d").to_string();

    TimeInfo { time, date }
}
