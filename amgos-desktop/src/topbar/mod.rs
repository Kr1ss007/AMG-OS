//! Top Global Menu Panel Subsystem
//!
//! Position: Top of screen, full width, fixed height (32px), never hides.
//! Left: Space Orange (#FF5500) sharp rectangle mark (no logo, no wordmark).
//! Center: Global Menu for focused application.
//! Right: System Tray, Clock, Status Indicators (Network, Audio, Battery).

pub const TOPBAR_HEIGHT_PX: u32 = 32;
pub const IDENTITY_MARK_WIDTH_PX: u32 = 36;
pub const IDENTITY_MARK_HEIGHT_PX: u32 = 24;
pub const SPACE_ORANGE_HEX: &str = "#FF5500";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityMark {
    pub color_hex: &'static str,
    pub width_px: u32,
    pub height_px: u32,
    pub corner_radius_px: u32, // Strictly 0 (perfect rectangle, sharp corners)
}

impl Default for IdentityMark {
    fn default() -> Self {
        Self {
            color_hex: SPACE_ORANGE_HEX,
            width_px: IDENTITY_MARK_WIDTH_PX,
            height_px: IDENTITY_MARK_HEIGHT_PX,
            corner_radius_px: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopGlobalMenuBar {
    pub identity_mark: IdentityMark,
    pub active_app_title: String,
    pub menu_items: Vec<String>,
    pub network_connected: bool,
    pub audio_volume_percent: u8,
    pub battery_percent: Option<u8>,
    pub clock_display: String,
}

impl TopGlobalMenuBar {
    pub fn new() -> Self {
        Self {
            identity_mark: IdentityMark::default(),
            active_app_title: "Filer".to_string(),
            menu_items: vec![
                "File".to_string(),
                "Edit".to_string(),
                "View".to_string(),
                "Go".to_string(),
                "Window".to_string(),
                "Help".to_string(),
            ],
            network_connected: false,
            audio_volume_percent: 75,
            battery_percent: None, // Desktop / plugged-in
            clock_display: "14:22".to_string(),
        }
    }

    pub fn set_focused_app(&mut self, app_name: &str, menus: Vec<String>) {
        self.active_app_title = app_name.to_string();
        self.menu_items = menus;
    }

    pub fn update_network_status(&mut self, connected: bool) {
        self.network_connected = connected;
    }
}

impl Default for TopGlobalMenuBar {
    fn default() -> Self {
        Self::new()
    }
}
