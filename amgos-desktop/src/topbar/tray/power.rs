//! Power & Battery Tray Indicator Subsystem
//!
//! Subscribes to Power Manager events from Process 1.
//! Displays live battery level, charging/AC power state, and pre-configured
//! power profiles: Endurance, Balanced, and MAX.
//! Allows the user to switch active power profiles directly from the Top Panel.

use amgos_protocol::ebus::{DesktopRequest, PowerProfile, SystemEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerTrayWidget {
    pub active_profile: PowerProfile,
    pub active_governor: String,
    pub battery_pct: Option<u8>,
    pub is_ac_online: bool,
    pub is_charging: bool,
    pub popover_visible: bool,
}

impl PowerTrayWidget {
    pub fn new() -> Self {
        Self {
            active_profile: PowerProfile::Balanced,
            active_governor: "balance_performance".to_string(),
            battery_pct: Some(95),
            is_ac_online: true,
            is_charging: false,
            popover_visible: false,
        }
    }

    /// Update state from e-bus SystemEvents
    pub fn handle_event(&mut self, event: &SystemEvent) {
        if let SystemEvent::PowerProfileChanged {
            profile,
            active_governor,
        } = event
        {
            self.active_profile = *profile;
            self.active_governor = active_governor.clone();
        }
    }

    /// Produce DesktopRequest when the user selects a new power profile
    pub fn select_profile(&mut self, profile: PowerProfile) -> DesktopRequest {
        self.active_profile = profile;
        DesktopRequest::SetPowerProfile(profile)
    }

    /// Determine icon name for WhiteSur icon theme
    pub fn icon_name(&self) -> &'static str {
        match (self.battery_pct, self.is_charging, self.is_ac_online) {
            (None, _, _) => "ac-adapter",
            (Some(_), true, _) => "battery-charging",
            (Some(pct), false, _) if pct > 90 => "battery-100",
            (Some(pct), false, _) if pct > 70 => "battery-080",
            (Some(pct), false, _) if pct > 50 => "battery-060",
            (Some(pct), false, _) if pct > 30 => "battery-040",
            (Some(pct), false, _) if pct > 10 => "battery-020",
            _ => "battery-caution",
        }
    }

    /// Tooltip text describing power profile and battery charge
    pub fn tooltip_text(&self) -> String {
        let profile_str = match self.active_profile {
            PowerProfile::Endurance => "Endurance (Power Saver)",
            PowerProfile::Balanced => "Balanced",
            PowerProfile::Max => "MAX (Full Performance)",
        };

        match self.battery_pct {
            Some(pct) => {
                let status = if self.is_charging {
                    "Charging"
                } else if self.is_ac_online {
                    "AC Connected (Full)"
                } else {
                    "On Battery"
                };
                format!("Battery: {}% ({})\nProfile: {}", pct, status, profile_str)
            }
            None => format!("Power: AC Desktop\nProfile: {}", profile_str),
        }
    }

    pub fn toggle_popover(&mut self) {
        self.popover_visible = !self.popover_visible;
    }
}

impl Default for PowerTrayWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_tray_profile_and_icons() {
        let mut widget = PowerTrayWidget::new();
        assert_eq!(widget.active_profile, PowerProfile::Balanced);

        // Switch to MAX profile
        let req = widget.select_profile(PowerProfile::Max);
        assert_eq!(req, DesktopRequest::SetPowerProfile(PowerProfile::Max));
        assert_eq!(widget.active_profile, PowerProfile::Max);

        // Battery icon variations
        widget.battery_pct = Some(85);
        widget.is_charging = false;
        assert_eq!(widget.icon_name(), "battery-080");

        widget.is_charging = true;
        assert_eq!(widget.icon_name(), "battery-charging");

        widget.battery_pct = None;
        assert_eq!(widget.icon_name(), "ac-adapter");
    }
}
