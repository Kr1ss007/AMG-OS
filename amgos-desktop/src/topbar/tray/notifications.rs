//! Notification Center Tray Widget
//!
//! Tracks incoming system and third-party notifications.
//! Renders the unread badge count, Do Not Disturb (DND) status,
//! and toggles the system Notification Center drawer.

use amgos_protocol::ebus::SystemEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayNotificationItem {
    pub id: u64,
    pub app_id: String,
    pub title: String,
    pub body: String,
    pub urgency: u8,
    pub is_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NotificationTrayWidget {
    pub unread_count: u32,
    pub is_dnd_enabled: bool,
    pub drawer_open: bool,
    pub items: Vec<TrayNotificationItem>,
}

impl NotificationTrayWidget {
    pub fn new() -> Self {
        Self {
            unread_count: 0,
            is_dnd_enabled: false,
            drawer_open: false,
            items: Vec::new(),
        }
    }

    /// Process e-bus notification dispatch events
    pub fn handle_event(&mut self, event: &SystemEvent) {
        if let SystemEvent::NotificationDispatched {
            notification_id,
            app_id,
            title,
            body,
            urgency,
        } = event
        {
            let item = TrayNotificationItem {
                id: *notification_id,
                app_id: app_id.clone(),
                title: title.clone(),
                body: body.clone(),
                urgency: *urgency,
                is_read: false,
            };

            self.items.insert(0, item);
            if !self.is_dnd_enabled {
                self.unread_count = self.unread_count.saturating_add(1);
            }
        }
    }

    /// Mark all as read when opening drawer
    pub fn mark_all_read(&mut self) {
        self.unread_count = 0;
        for item in &mut self.items {
            item.is_read = true;
        }
    }

    /// Dismiss single notification
    pub fn dismiss(&mut self, id: u64) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            if !self.items[pos].is_read && self.unread_count > 0 {
                self.unread_count -= 1;
            }
            self.items.remove(pos);
        }
    }

    /// Clear all notifications
    pub fn clear_all(&mut self) {
        self.items.clear();
        self.unread_count = 0;
    }

    /// Toggle Do Not Disturb mode
    pub fn toggle_dnd(&mut self) {
        self.is_dnd_enabled = !self.is_dnd_enabled;
    }

    /// Toggle notification drawer visibility
    pub fn toggle_drawer(&mut self) {
        self.drawer_open = !self.drawer_open;
        if self.drawer_open {
            self.mark_all_read();
        }
    }

    /// Icon asset name for WhiteSur icon theme
    pub fn icon_name(&self) -> &'static str {
        if self.is_dnd_enabled {
            "notification-dnd"
        } else if self.unread_count > 0 {
            "notification-active"
        } else {
            "notification-none"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_tray_events_and_read_state() {
        let mut widget = NotificationTrayWidget::new();
        assert_eq!(widget.unread_count, 0);
        assert_eq!(widget.icon_name(), "notification-none");

        let event = SystemEvent::NotificationDispatched {
            notification_id: 101,
            app_id: "zen".to_string(),
            title: "Download Complete".to_string(),
            body: "amgos-package.deb downloaded".to_string(),
            urgency: 1,
        };
        widget.handle_event(&event);

        assert_eq!(widget.unread_count, 1);
        assert_eq!(widget.icon_name(), "notification-active");
        assert_eq!(widget.items.len(), 1);

        // Open drawer marks all read
        widget.toggle_drawer();
        assert!(widget.drawer_open);
        assert_eq!(widget.unread_count, 0);

        // Toggle DND
        widget.toggle_dnd();
        assert!(widget.is_dnd_enabled);
        assert_eq!(widget.icon_name(), "notification-dnd");
    }
}
