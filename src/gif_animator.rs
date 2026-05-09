use crate::gif_loader::GifLoader;
use slint::{Timer, TimerMode, Weak};
use std::time::Duration;

const MAX_FRAMES: usize = 2;

pub struct GifAnimator {
    frames: Vec<slint::Image>,
}

impl GifAnimator {
    pub fn new() -> Self {
        let frames = GifLoader::new()
            .map(|l: GifLoader| l.into_frames())
            .unwrap_or_default();
        Self { frames }
    }

    pub fn init(&self, app: &crate::StatusBarWindow) {
        if let Some(frame) = self.frames.first() {
            app.set_frame_0(frame.clone());
        }
        if self.frames.len() > 1 {
            app.set_frame_1(self.frames[1].clone());
        }
        app.set_media_gif_frame_count(self.frames.len() as i32);
    }

    pub fn start_animation(&self, app_weak: Weak<crate::StatusBarWindow>, interval_ms: u64) -> Timer {
        let mut current_frame: usize = 0;
        let mut last_frame: usize = usize::MAX;
        let frame_count = self.frames.len().max(1).min(MAX_FRAMES);
        let app_weak_clone = app_weak.clone();

        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(interval_ms), move || {
            if let Some(window) = app_weak_clone.upgrade() {
                let status: slint::SharedString = window.get_media_status();

                if status.as_str() != "playing" {
                    if current_frame != 0 {
                        current_frame = 0;
                        last_frame = 0;
                        window.set_media_gif_current_frame_index(0);
                    }
                    return;
                }

                current_frame = (current_frame + 1) % frame_count;

                if current_frame != last_frame {
                    last_frame = current_frame;
                    window.set_media_gif_current_frame_index(current_frame as i32);
                }
            }
        });
        
        timer
    }
}

impl Default for GifAnimator {
    fn default() -> Self {
        Self::new()
    }
}