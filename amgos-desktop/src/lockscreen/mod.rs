//! Lockscreen Controller Subsystem
//!
//! Owns authentication UI presentation, time & date rendering in Inter Display,
//! and cubic-bezier fade transitions. Never holds password hashes locally.

use crate::animations::curves;

#[derive(Debug, Clone)]
pub struct LockscreenState {
    pub is_locked: bool,
    pub user_display_name: String,
    pub time_str: String,
    pub date_str: String,
    pub transition_progress: f64, // 0.0 to 1.0
}

impl LockscreenState {
    pub fn new(user_name: &str) -> Self {
        Self {
            is_locked: true,
            user_display_name: user_name.to_string(),
            time_str: "14:22".to_string(),
            date_str: "Monday, September 14, 2026".to_string(),
            transition_progress: 1.0,
        }
    }

    pub fn unlock_fade_opacity(&self) -> f64 {
        1.0 - curves::STANDARD.solve(self.transition_progress)
    }

    pub fn lock(&mut self) {
        self.is_locked = true;
        self.transition_progress = 1.0;
    }

    pub fn unlock(&mut self) {
        self.is_locked = false;
        self.transition_progress = 0.0;
    }
}
