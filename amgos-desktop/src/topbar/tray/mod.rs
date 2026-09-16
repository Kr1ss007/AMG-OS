//! StatusNotifierItem (SNI) Tray Protocol Host
//!
//! Implements the FreeDesktop / KDE StatusNotifierItem (org.kde.StatusNotifierItem)
//! and StatusNotifierWatcher (org.kde.StatusNotifierWatcher) host architecture.
//! Allows native and foreign applications to register dynamic tray items,
//! update status, provide icon pixmaps/names, tooltips, and receive activation events.

pub mod audio;
pub mod clock;
pub mod network;
pub mod notifications;
pub mod power;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// StatusNotifierItem category classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SniCategory {
    #[default]
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
    Other,
}

/// StatusNotifierItem status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SniStatus {
    Passive,
    #[default]
    Active,
    NeedsAttention,
}

/// ARGB32 icon pixmap data transmitted over D-Bus
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SniPixmap {
    pub width: u32,
    pub height: u32,
    pub bytes_argb32: Vec<u8>,
}

/// ToolTip structure provided by StatusNotifierItem
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SniToolTip {
    pub icon_name: String,
    pub icon_pixmaps: Vec<SniPixmap>,
    pub title: String,
    pub description: String,
}

/// Registered StatusNotifierItem state tracked by the host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusNotifierItem {
    pub service: String,
    pub path: String,
    pub id: String,
    pub title: String,
    pub category: SniCategory,
    pub status: SniStatus,
    pub icon_name: Option<String>,
    pub icon_pixmaps: Vec<SniPixmap>,
    pub overlay_icon_name: Option<String>,
    pub attention_icon_name: Option<String>,
    pub attention_movie_name: Option<String>,
    pub tooltip: SniToolTip,
    pub menu_path: Option<String>,
    pub window_id: u32,
}

impl StatusNotifierItem {
    pub fn new(service: &str, path: &str, id: &str) -> Self {
        Self {
            service: service.to_string(),
            path: path.to_string(),
            id: id.to_string(),
            title: id.to_string(),
            category: SniCategory::ApplicationStatus,
            status: SniStatus::Active,
            icon_name: None,
            icon_pixmaps: Vec::new(),
            overlay_icon_name: None,
            attention_icon_name: None,
            attention_movie_name: None,
            tooltip: SniToolTip::default(),
            menu_path: None,
            window_id: 0,
        }
    }
}

/// Host registry tracking all registered StatusNotifierItems
#[derive(Debug, Clone, Default)]
pub struct StatusNotifierHost {
    items: HashMap<String, StatusNotifierItem>,
    is_watcher_registered: bool,
}

impl StatusNotifierHost {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            is_watcher_registered: true,
        }
    }

    /// Register a new StatusNotifierItem by its D-Bus service and object path
    pub fn register_item(
        &mut self,
        service: &str,
        path: &str,
        id: &str,
    ) -> &mut StatusNotifierItem {
        let key = format!("{}:{}", service, path);
        let item = StatusNotifierItem::new(service, path, id);
        self.items.entry(key.clone()).or_insert(item);
        self.items.get_mut(&key).expect("Key was just inserted")
    }

    /// Unregister a StatusNotifierItem when its service disconnects
    pub fn unregister_item(&mut self, service: &str, path: &str) -> Option<StatusNotifierItem> {
        let key = format!("{}:{}", service, path);
        self.items.remove(&key)
    }

    /// Retrieve an item by service and path
    pub fn get_item(&self, service: &str, path: &str) -> Option<&StatusNotifierItem> {
        let key = format!("{}:{}", service, path);
        self.items.get(&key)
    }

    /// Retrieve a mutable item by service and path
    pub fn get_item_mut(&mut self, service: &str, path: &str) -> Option<&mut StatusNotifierItem> {
        let key = format!("{}:{}", service, path);
        self.items.get_mut(&key)
    }

    /// Get all currently registered items (for UI rendering in the tray)
    pub fn iter_items(&self) -> impl Iterator<Item = &StatusNotifierItem> {
        self.items.values()
    }

    /// Total number of active items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn is_watcher_registered(&self) -> bool {
        self.is_watcher_registered
    }

    /// Update status of a registered item
    pub fn update_status(&mut self, service: &str, path: &str, status: SniStatus) -> bool {
        if let Some(item) = self.get_item_mut(service, path) {
            item.status = status;
            true
        } else {
            false
        }
    }

    /// Update title of a registered item
    pub fn update_title(&mut self, service: &str, path: &str, title: &str) -> bool {
        if let Some(item) = self.get_item_mut(service, path) {
            item.title = title.to_string();
            true
        } else {
            false
        }
    }

    /// Update icon name of a registered item
    pub fn update_icon_name(&mut self, service: &str, path: &str, icon_name: &str) -> bool {
        if let Some(item) = self.get_item_mut(service, path) {
            item.icon_name = Some(icon_name.to_string());
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sni_host_registration_lifecycle() {
        let mut host = StatusNotifierHost::new();
        assert_eq!(host.len(), 0);

        let item = host.register_item("org.amgos.zen", "/StatusNotifierItem", "zen-browser");
        item.icon_name = Some("zen-browser".to_string());
        item.category = SniCategory::Communications;

        assert_eq!(host.len(), 1);
        let queried = host
            .get_item("org.amgos.zen", "/StatusNotifierItem")
            .unwrap();
        assert_eq!(queried.id, "zen-browser");
        assert_eq!(queried.icon_name.as_deref(), Some("zen-browser"));
        assert_eq!(queried.status, SniStatus::Active);

        // Update status
        assert!(host.update_status(
            "org.amgos.zen",
            "/StatusNotifierItem",
            SniStatus::NeedsAttention
        ));
        assert_eq!(
            host.get_item("org.amgos.zen", "/StatusNotifierItem")
                .unwrap()
                .status,
            SniStatus::NeedsAttention
        );

        // Unregister
        let removed = host.unregister_item("org.amgos.zen", "/StatusNotifierItem");
        assert!(removed.is_some());
        assert_eq!(host.len(), 0);
    }
}
