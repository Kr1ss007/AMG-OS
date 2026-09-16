//! Network Tray Indicator Subsystem
//!
//! Tracks real-time network connectivity, interface states (Wi-Fi, Ethernet),
//! active SSID, IP addressing, and RSSI signal quality.
//! Renders visual state in the Top Panel system tray and provides Wi-Fi selection popover.

use amgos_protocol::ebus::{SystemEvent, WifiAccessPoint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkKind {
    Wifi,
    Ethernet,
    Cellular,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTrayWidget {
    pub is_connected: bool,
    pub interface_name: String,
    pub kind: NetworkKind,
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
    pub signal_pct: u8,
    pub scanned_aps: Vec<WifiAccessPoint>,
    pub popover_visible: bool,
}

impl NetworkTrayWidget {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            interface_name: String::new(),
            kind: NetworkKind::Disconnected,
            ssid: None,
            ip_address: None,
            signal_pct: 0,
            scanned_aps: Vec::new(),
            popover_visible: false,
        }
    }

    /// Update widget state from e-bus SystemEvent
    pub fn handle_event(&mut self, event: &SystemEvent) {
        match event {
            SystemEvent::NetworkStateChanged {
                connected,
                interface_name,
                ssid,
                ip_address,
            } => {
                self.is_connected = *connected;
                self.interface_name = interface_name.clone();
                self.ssid = ssid.clone();
                self.ip_address = ip_address.clone();

                if !*connected {
                    self.kind = NetworkKind::Disconnected;
                    self.signal_pct = 0;
                } else if interface_name.starts_with('w') {
                    self.kind = NetworkKind::Wifi;
                    // Default baseline signal if not yet populated by scan
                    if self.signal_pct == 0 {
                        self.signal_pct = 85;
                    }
                } else {
                    self.kind = NetworkKind::Ethernet;
                    self.signal_pct = 100;
                }
            }
            SystemEvent::WifiScanResults { access_points } => {
                self.scanned_aps = access_points.clone();
                // Update signal_pct if current connected SSID is in the scan list
                if let Some(ref current_ssid) = self.ssid {
                    if let Some(ap) = access_points.iter().find(|a| &a.ssid == current_ssid) {
                        self.signal_pct = ap.signal_strength_pct;
                    }
                }
            }
            _ => {}
        }
    }

    /// Discrete signal bars representation (0 to 4)
    pub fn signal_bars(&self) -> u8 {
        if !self.is_connected || self.kind == NetworkKind::Disconnected {
            0
        } else if self.kind == NetworkKind::Ethernet {
            4
        } else {
            match self.signal_pct {
                0..=10 => 0,
                11..=35 => 1,
                36..=60 => 2,
                61..=85 => 3,
                _ => 4,
            }
        }
    }

    /// Text tooltip description for the top bar
    pub fn tooltip_text(&self) -> String {
        if !self.is_connected {
            "Network: Disconnected".to_string()
        } else if let Some(ref ssid) = self.ssid {
            let ip = self.ip_address.as_deref().unwrap_or("No IP");
            format!("Wi-Fi: {} ({}%)\nIP: {}", ssid, self.signal_pct, ip)
        } else {
            let ip = self.ip_address.as_deref().unwrap_or("No IP");
            format!("Ethernet: Connected ({})\nIP: {}", self.interface_name, ip)
        }
    }

    pub fn toggle_popover(&mut self) {
        self.popover_visible = !self.popover_visible;
    }
}

impl Default for NetworkTrayWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_tray_events_and_signal_bars() {
        let mut widget = NetworkTrayWidget::new();
        assert_eq!(widget.signal_bars(), 0);
        assert!(!widget.is_connected);

        // Connected to Wi-Fi
        let event = SystemEvent::NetworkStateChanged {
            connected: true,
            interface_name: "wlo1".to_string(),
            ssid: Some("AMGOS-5G".to_string()),
            ip_address: Some("192.168.1.105".to_string()),
        };
        widget.handle_event(&event);
        assert!(widget.is_connected);
        assert_eq!(widget.kind, NetworkKind::Wifi);
        assert_eq!(widget.signal_bars(), 3); // Default 85% -> 3 bars

        // Scan results update signal to 95% -> 4 bars
        let scan = SystemEvent::WifiScanResults {
            access_points: vec![WifiAccessPoint {
                ssid: "AMGOS-5G".to_string(),
                signal_strength_pct: 95,
                is_secured: true,
            }],
        };
        widget.handle_event(&scan);
        assert_eq!(widget.signal_pct, 95);
        assert_eq!(widget.signal_bars(), 4);

        // Disconnect
        let disc = SystemEvent::NetworkStateChanged {
            connected: false,
            interface_name: "wlo1".to_string(),
            ssid: None,
            ip_address: None,
        };
        widget.handle_event(&disc);
        assert!(!widget.is_connected);
        assert_eq!(widget.signal_bars(), 0);
    }
}
