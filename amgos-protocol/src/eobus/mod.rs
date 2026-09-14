//! AMGOS EventOutsiderBus (eo-bus) Module
//!
//! Owns external D-Bus bridge communication and contracts for third-party applications.
//!
//! Rule: Third-party apps communicate strictly over D-Bus via EventOutsiderBus.
//! They never have access to internal pub/sub (EventBus / e-bus).
//! EventOutsiderBus handles translation and sanitization.

use serde::{Deserialize, Serialize};

/// Global Menu item definition exported over D-Bus (`org.amgos.GlobalMenu`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: u32,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub shortcut: Option<String>,
    pub children: Vec<MenuItem>,
}

/// Menu layout representation for foreign apps (GTK, Qt, Electron)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuLayout {
    pub app_id: String,
    pub window_id: u32,
    pub root_items: Vec<MenuItem>,
}

/// External notification dispatched by third-party applications
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalNotification {
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub timeout_ms: i32,
}

/// External application metadata registered with the desktop shell
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAppMetadata {
    pub desktop_file_id: String,
    pub display_name: String,
    pub executable_path: String,
    pub icon_name: String,
    pub supports_global_menu: bool,
    pub supports_wayland_native: bool,
}

/// D-Bus interface names standardized in AMGOS
pub mod dbus_interfaces {
    pub const GLOBAL_MENU_INTERFACE: &str = "org.amgos.GlobalMenu";
    pub const NOTIFICATIONS_INTERFACE: &str = "org.freedesktop.Notifications";
    pub const MPRIS_INTERFACE: &str = "org.mpris.MediaPlayer2";
    pub const STATUS_NOTIFIER_INTERFACE: &str = "org.kde.StatusNotifierItem";
}
