//! Top Global Menu Panel & System Tray Subsystem
//!
//! Specifications from AMGOS_INTERFACE_SHELL_v0.0.1 Section 3.1:
//! - Position: Top of screen, full width, fixed height (32px), never hides.
//! - Left: Space Orange (#FF5500) sharp rectangle mark (no logo, no wordmark).
//!   Clicking opens the system menu.
//! - Center / Remainder: Global Menu for the focused application (native or foreign).
//! - Right: Real System Tray:
//!   * StatusNotifierItem (SNI) host for application tray icons.
//!   * Live Network status indicator & Wi-Fi popover.
//!   * Live AVM Audio volume indicator, ducking state & slider.
//!   * Live Power & Battery monitor with Endurance / Balanced / MAX switcher.
//!   * Clock & Calendar display.
//!   * Notification Center access icon with unread badge counter.

pub mod tray;

pub use tray::audio::AudioTrayWidget;
pub use tray::clock::ClockTrayWidget;
pub use tray::network::NetworkTrayWidget;
pub use tray::notifications::NotificationTrayWidget;
pub use tray::power::PowerTrayWidget;
pub use tray::{SniCategory, SniStatus, StatusNotifierHost, StatusNotifierItem};

use amgos_protocol::ebus::SystemEvent;
use serde::{Deserialize, Serialize};

pub const TOPBAR_HEIGHT_PX: u32 = 32;
pub const IDENTITY_MARK_WIDTH_PX: u32 = 36;
pub const IDENTITY_MARK_HEIGHT_PX: u32 = 24;
pub const SPACE_ORANGE_HEX: &str = "#FF5500";

/// Identity mark at the far left of the Top Global Menu Panel.
/// Strictly a sharp rectangle with 0 corner radius.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityMark {
    pub color_hex: &'static str,
    pub width_px: u32,
    pub height_px: u32,
    pub corner_radius_px: u32, // Strictly 0: no rounded corners
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

/// Global menu for the currently focused application
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusedAppMenu {
    pub app_title: String,
    pub menu_items: Vec<String>,
}

impl Default for FocusedAppMenu {
    fn default() -> Self {
        Self {
            app_title: "Filer".to_string(),
            menu_items: vec![
                "File".to_string(),
                "Edit".to_string(),
                "View".to_string(),
                "Go".to_string(),
                "Window".to_string(),
                "Help".to_string(),
            ],
        }
    }
}

/// Clickable targets on the Top Global Menu Panel for hit-testing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopBarTarget {
    IdentityMark,
    AppMenu(usize),
    TraySniItem(String),
    TrayNetwork,
    TrayAudio,
    TrayPower,
    TrayClock,
    TrayNotifications,
}

/// Complete Top Global Menu Panel & System Tray Subsystem
pub type TopGlobalMenuBar = TopGlobalMenuPanel;

#[derive(Debug, Clone)]
pub struct TopGlobalMenuPanel {
    pub identity_mark: IdentityMark,
    pub system_menu_open: bool,
    pub focused_app: FocusedAppMenu,
    pub sni_host: StatusNotifierHost,
    pub network: NetworkTrayWidget,
    pub audio: AudioTrayWidget,
    pub power: PowerTrayWidget,
    pub clock: ClockTrayWidget,
    pub notifications: NotificationTrayWidget,
}

impl TopGlobalMenuPanel {
    pub fn new() -> Self {
        Self {
            identity_mark: IdentityMark::default(),
            system_menu_open: false,
            focused_app: FocusedAppMenu::default(),
            sni_host: StatusNotifierHost::new(),
            network: NetworkTrayWidget::new(),
            audio: AudioTrayWidget::new(),
            power: PowerTrayWidget::new(),
            clock: ClockTrayWidget::new(),
            notifications: NotificationTrayWidget::new(),
        }
    }

    /// Dispatch e-bus SystemEvents to all child widgets in the tray
    pub fn handle_system_event(&mut self, event: &SystemEvent) {
        self.network.handle_event(event);
        self.audio.handle_event(event);
        self.power.handle_event(event);
        self.notifications.handle_event(event);

        if let SystemEvent::ForeignAppMenuUpdated { app_id, menu_json } = event {
            if let Ok(items) = serde_json::from_str::<Vec<String>>(menu_json) {
                self.focused_app.app_title = app_id.clone();
                self.focused_app.menu_items = items;
            }
        }
    }

    /// Set the active focused application title and top-level menus
    pub fn set_focused_app(&mut self, app_name: &str, menus: Vec<String>) {
        self.focused_app.app_title = app_name.to_string();
        self.focused_app.menu_items = menus;
    }

    /// Toggle the Space Orange identity mark system menu
    pub fn toggle_system_menu(&mut self) {
        self.system_menu_open = !self.system_menu_open;
    }

    /// Helper to directly update network connection status
    pub fn update_network_status(&mut self, connected: bool) {
        self.network.is_connected = connected;
    }

    /// Hit-testing for cursor clicks along the 32px top panel
    pub fn hit_test(&self, x: u32, screen_width: u32) -> Option<TopBarTarget> {
        // Identity mark: x in [8, 8 + IDENTITY_MARK_WIDTH_PX]
        if (8..=(8 + IDENTITY_MARK_WIDTH_PX)).contains(&x) {
            return Some(TopBarTarget::IdentityMark);
        }

        // Global menu starts at x = 60
        let mut curr_x = 60;
        for (idx, item) in self.focused_app.menu_items.iter().enumerate() {
            let item_width = (item.len() as u32 * 9) + 16;
            if x >= curr_x && x < curr_x + item_width {
                return Some(TopBarTarget::AppMenu(idx));
            }
            curr_x += item_width;
        }

        // System tray occupies the right side
        // Layout: [SNI Items] [Network] [Audio] [Power] [Clock] [Notifications]
        // Notifications: [screen_width - 36, screen_width - 8]
        if x >= screen_width.saturating_sub(36) && x <= screen_width.saturating_sub(8) {
            return Some(TopBarTarget::TrayNotifications);
        }

        // Clock: [screen_width - 170, screen_width - 40]
        if x >= screen_width.saturating_sub(170) && x < screen_width.saturating_sub(40) {
            return Some(TopBarTarget::TrayClock);
        }

        // Power: [screen_width - 210, screen_width - 174]
        if x >= screen_width.saturating_sub(210) && x < screen_width.saturating_sub(174) {
            return Some(TopBarTarget::TrayPower);
        }

        // Audio: [screen_width - 250, screen_width - 214]
        if x >= screen_width.saturating_sub(250) && x < screen_width.saturating_sub(214) {
            return Some(TopBarTarget::TrayAudio);
        }

        // Network: [screen_width - 290, screen_width - 254]
        if x >= screen_width.saturating_sub(290) && x < screen_width.saturating_sub(254) {
            return Some(TopBarTarget::TrayNetwork);
        }

        None
    }
}

impl Default for TopGlobalMenuPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topbar_initialization_and_identity_mark() {
        let panel = TopGlobalMenuPanel::new();
        assert_eq!(panel.identity_mark.color_hex, "#FF5500");
        assert_eq!(panel.identity_mark.corner_radius_px, 0);
        assert_eq!(panel.focused_app.app_title, "Filer");
        assert_eq!(panel.focused_app.menu_items.len(), 6);
    }

    #[test]
    fn test_topbar_system_event_propagation() {
        let mut panel = TopGlobalMenuPanel::new();

        let net_ev = SystemEvent::NetworkStateChanged {
            connected: true,
            interface_name: "wlo1".to_string(),
            ssid: Some("Office-WiFi".to_string()),
            ip_address: Some("10.0.0.42".to_string()),
        };
        panel.handle_system_event(&net_ev);
        assert!(panel.network.is_connected);
        assert_eq!(panel.network.ssid.as_deref(), Some("Office-WiFi"));

        let notif_ev = SystemEvent::NotificationDispatched {
            notification_id: 1,
            app_id: "terminow".to_string(),
            title: "Build Succeeded".to_string(),
            body: "AMGOS compiled in 4.2s".to_string(),
            urgency: 0,
        };
        panel.handle_system_event(&notif_ev);
        assert_eq!(panel.notifications.unread_count, 1);
    }

    #[test]
    fn test_topbar_hit_testing() {
        let panel = TopGlobalMenuPanel::new();
        let screen_width = 1920;

        // Hit identity mark
        assert_eq!(panel.hit_test(16, screen_width), Some(TopBarTarget::IdentityMark));

        // Hit notifications at right
        assert_eq!(
            panel.hit_test(screen_width - 20, screen_width),
            Some(TopBarTarget::TrayNotifications)
        );

        // Hit clock
        assert_eq!(
            panel.hit_test(screen_width - 100, screen_width),
            Some(TopBarTarget::TrayClock)
        );
    }
}
