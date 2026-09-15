//! EventOutsiderBus (eo-bus) Subsystem in Process 1
//!
//! Exposes sanitized external interfaces over D-Bus to third-party applications.
//! Completely isolated from internal EventBus.
//! Handles MenuLayout translation, Notification forwarding, and foreign app lifecycle.

pub use amgos_protocol::eobus::*;

use amgos_protocol::ebus::SystemEvent;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct EventOutsiderBusBridge {
    active_menus: Arc<Mutex<HashMap<String, MenuLayout>>>,
    active_notifications: Arc<Mutex<VecDeque<(u64, ExternalNotification)>>>,
    notification_id_seq: AtomicU64,
}

impl EventOutsiderBusBridge {
    pub fn new() -> Self {
        Self {
            active_menus: Arc::new(Mutex::new(HashMap::new())),
            active_notifications: Arc::new(Mutex::new(VecDeque::new())),
            notification_id_seq: AtomicU64::new(1),
        }
    }

    /// Register or update a foreign application's menu from D-Bus
    pub fn update_foreign_menu(&self, layout: MenuLayout) -> SystemEvent {
        let app_id = layout.app_id.clone();
        let menu_json = serde_json::to_string(&layout).unwrap_or_else(|_| "{}".to_string());

        let mut menus = self.active_menus.lock().unwrap();
        menus.insert(app_id.clone(), layout);

        SystemEvent::ForeignAppMenuUpdated { app_id, menu_json }
    }

    /// Receive and queue a notification from an external application
    pub fn handle_external_notification(&self, notif: ExternalNotification) -> SystemEvent {
        let notification_id = self.notification_id_seq.fetch_add(1, Ordering::SeqCst);
        let app_id = notif.app_name.clone();
        let title = notif.summary.clone();
        let body = notif.body.clone();
        let urgency = 1u8; // Normal urgency

        let mut queue = self.active_notifications.lock().unwrap();
        if queue.len() >= 500 {
            queue.pop_front();
        }
        queue.push_back((notification_id, notif));

        SystemEvent::NotificationDispatched {
            notification_id,
            app_id,
            title,
            body,
            urgency,
        }
    }

    /// Dismiss a notification by ID
    pub fn dismiss_notification(&self, notification_id: u64) {
        let mut queue = self.active_notifications.lock().unwrap();
        queue.retain(|(id, _)| *id != notification_id);
    }

    /// Get current menu layout for an app
    pub fn get_menu(&self, app_id: &str) -> Option<MenuLayout> {
        let menus = self.active_menus.lock().unwrap();
        menus.get(app_id).cloned()
    }
}

impl Default for EventOutsiderBusBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eobus_foreign_menu_and_notifications() {
        let bridge = EventOutsiderBusBridge::new();

        // Foreign Menu update
        let layout = MenuLayout {
            app_id: "zen-browser".to_string(),
            window_id: 42,
            root_items: vec![MenuItem {
                id: 1,
                label: "File".to_string(),
                enabled: true,
                visible: true,
                shortcut: None,
                children: vec![MenuItem {
                    id: 2,
                    label: "New Tab".to_string(),
                    enabled: true,
                    visible: true,
                    shortcut: Some("Ctrl+T".to_string()),
                    children: Vec::new(),
                }],
            }],
        };

        let event = bridge.update_foreign_menu(layout);
        match event {
            SystemEvent::ForeignAppMenuUpdated { app_id, menu_json } => {
                assert_eq!(app_id, "zen-browser");
                assert!(menu_json.contains("New Tab"));
            }
            _ => panic!("Expected ForeignAppMenuUpdated event"),
        }

        // Notification dispatch
        let notif = ExternalNotification {
            app_name: "zen-browser".to_string(),
            replaces_id: 0,
            app_icon: "zen".to_string(),
            summary: "Download Complete".to_string(),
            body: "AMGOS-v0.0.1.iso finished downloading".to_string(),
            actions: Vec::new(),
            timeout_ms: 5000,
        };

        let notif_event = bridge.handle_external_notification(notif);
        match notif_event {
            SystemEvent::NotificationDispatched {
                notification_id,
                app_id,
                title,
                body,
                ..
            } => {
                assert_eq!(notification_id, 1);
                assert_eq!(app_id, "zen-browser");
                assert_eq!(title, "Download Complete");
                assert!(body.contains("AMGOS-v0.0.1.iso"));
            }
            _ => panic!("Expected NotificationDispatched event"),
        }
    }
}
