//! Shutdown and Restart UI Surfaces
//!
//! Owned by Animation and Interface Engine in Process 2.
//! Black background. Inter font. Lowercase centered text.
//! - Shutdown: "goodbye"
//! - Restart: "i'll see you in a bit"
//!
//! Governed by cubic-bezier timing curves.

use crate::animations::curves;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerScreenType {
    Shutdown,
    Restart,
}

#[derive(Debug, Clone)]
pub struct PowerScreenState {
    pub screen_type: PowerScreenType,
    pub background_color_hex: &'static str,
    pub text_color_hex: &'static str,
    pub progress: f64, // 0.0 to 1.0
}

impl PowerScreenState {
    pub fn new(screen_type: PowerScreenType) -> Self {
        Self {
            screen_type,
            background_color_hex: "#000000",
            text_color_hex: "#FFFFFF",
            progress: 0.0,
        }
    }

    pub fn display_text(&self) -> &'static str {
        match self.screen_type {
            PowerScreenType::Shutdown => "goodbye",
            PowerScreenType::Restart => "i'll see you in a bit",
        }
    }

    pub fn text_opacity(&self) -> f64 {
        curves::DECELERATE.solve(self.progress)
    }

    pub fn set_progress(&mut self, p: f64) {
        self.progress = p.clamp(0.0, 1.0);
    }
}
