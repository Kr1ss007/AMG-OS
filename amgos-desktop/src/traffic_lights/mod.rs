//! Traffic Light Window Controls
//!
//! Position: Top left corner of every window chrome.
//! Three circular buttons:
//! - Close: Space Orange (#FF5500), 'X' symbol.
//! - Minimize: Space White (#F0F0F2), '-' symbol.
//! - Maximize: Sky Blue (#00A3FF), '+' symbol.
//!
//! Rule: Symbols hidden by default. Hovering any part of the button group
//! reveals all three symbols simultaneously.

pub const COLOR_CLOSE: &str = "#FF5500"; // Space Orange
pub const COLOR_MINIMIZE: &str = "#F0F0F2"; // Space White
pub const COLOR_MAXIMIZE: &str = "#00A3FF"; // Sky Blue

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControlAction {
    Close,
    Minimize,
    Maximize,
}

#[derive(Debug, Clone)]
pub struct TrafficLightGroup {
    pub is_group_hovered: bool,
    pub diameter_px: u32,
    pub spacing_px: u32,
}

impl TrafficLightGroup {
    pub fn new() -> Self {
        Self {
            is_group_hovered: false,
            diameter_px: 12,
            spacing_px: 8,
        }
    }

    pub fn set_hover(&mut self, hovered: bool) {
        self.is_group_hovered = hovered;
    }

    pub fn symbols_visible(&self) -> bool {
        self.is_group_hovered
    }

    pub fn close_symbol(&self) -> Option<char> {
        if self.symbols_visible() {
            Some('✕')
        } else {
            None
        }
    }

    pub fn minimize_symbol(&self) -> Option<char> {
        if self.symbols_visible() {
            Some('–')
        } else {
            None
        }
    }

    pub fn maximize_symbol(&self) -> Option<char> {
        if self.symbols_visible() {
            Some('+')
        } else {
            None
        }
    }
}

impl Default for TrafficLightGroup {
    fn default() -> Self {
        Self::new()
    }
}
