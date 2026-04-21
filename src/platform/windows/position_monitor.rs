#![allow(dead_code)]

use super::appbar::{force_window_position, get_window_position, get_work_area, AppBar};
use super::config::AppBarEdge;

pub fn start_position_monitor(hwnd: isize, config_height: i32) {
    println!("[position_monitor] Starting...");

    use std::sync::atomic::{AtomicBool, Ordering};

    let running = std::sync::Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    std::thread::spawn(move || {
        let mut failures = 0i32;
        let mut re_register_attempts = 0i32;
        let mut stable_cycles = 0i32;
        let mut bar = AppBar::new();
        bar.set_window_style(true);

        println!("[position_monitor] Waiting for Slint to render...");
        std::thread::sleep(std::time::Duration::from_millis(200));

        while running_clone.load(Ordering::Relaxed) {
            let rect = get_window_position(hwnd);
            let work_area = get_work_area();

            let work_area_ok = work_area.top == config_height;
            let window_ok = rect.left == 0 && rect.top == 0;
            let needs_reposition = !window_ok && !work_area_ok;

            if needs_reposition {
                println!(
                    "[position_monitor] Repositioning: ({}, {}) -> (0, 0)",
                    rect.left, rect.top
                );
                force_window_position(hwnd, 0, 0, 1366, config_height);
                stable_cycles = 0;
            }

            if !work_area_ok {
                failures += 1;
                stable_cycles = 0;
                println!(
                    "[position_monitor] Work area LOST (top={}), failures={}",
                    work_area.top, failures
                );

                if failures >= 2 {
                    re_register_attempts += 1;
                    println!(
                        "[position_monitor] Re-registering AppBar (attempt {})...",
                        re_register_attempts
                    );

                    bar.unregister();
                    std::thread::sleep(std::time::Duration::from_millis(20));

                    super::appbar::kill_systray_timer();

                    let registered = bar.register(hwnd, AppBarEdge::Top, config_height);
                    if registered {
                        println!("[position_monitor] AppBar re-registered successfully");
                        bar.notify_pos_changed();
                    }

                    force_window_position(hwnd, 0, 0, 1366, config_height);

                    std::thread::sleep(std::time::Duration::from_millis(50));

                    let new_work_area = get_work_area();
                    if new_work_area.top == config_height {
                        println!(
                            "[position_monitor] SUCCESS! Work area restored (top={})",
                            new_work_area.top
                        );
                        failures = 0;
                        re_register_attempts = 0;
                    } else {
                        println!(
                            "[position_monitor] FAILED! Work area still lost (top={})",
                            new_work_area.top
                        );
                        failures = 1;
                        if re_register_attempts >= 3 {
                            println!("[position_monitor] Max attempts reached, resetting");
                            re_register_attempts = 0;
                            failures = 0;
                        }
                    }
                }
            } else {
                stable_cycles += 1;
                if failures > 0 || stable_cycles == 1 {
                    println!("[position_monitor] Work area OK (top={})", work_area.top);
                }
                failures = 0;
                re_register_attempts = 0;
            }

            let sleep_duration = if stable_cycles >= 10 {
                println!("[position_monitor] Stable, sleeping 30s...");
                std::time::Duration::from_millis(30000)
            } else {
                std::time::Duration::from_millis(500)
            };

            std::thread::sleep(sleep_duration);
        }

        println!("[position_monitor] Stopped");
    });
}