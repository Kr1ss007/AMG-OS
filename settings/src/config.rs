//! System Configuration Model
//!
//! Owns user-configurable parameters across all topics:
//! Display, Sound, Network, Touchpad, Keyboard, Accessibility, Accounts, Privacy, Appearance.
//! Serializes into binary payload signed for Process 1 permission manager.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemConfiguration {
    pub display: DisplayConfig,
    pub sound: SoundConfig,
    pub touchpad: TouchpadConfig,
    pub keyboard: KeyboardConfig,
    pub accessibility: AccessibilityConfig,
    pub appearance: AppearanceConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub scale_factor: f32,
    pub refresh_rate_hz: u32,
    pub night_light_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundConfig {
    pub master_volume: u8,
    pub boot_chime_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TouchpadConfig {
    pub tap_to_click: bool,
    pub natural_scrolling: bool,
    pub sensitivity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyboardConfig {
    pub layout: String,
    pub repeat_delay_ms: u32,
    pub repeat_interval_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    pub high_contrast: bool,
    pub large_text: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub dark_mode: bool,
    pub accent_color_hex: String,
}

impl Default for SystemConfiguration {
    fn default() -> Self {
        Self {
            display: DisplayConfig {
                scale_factor: 1.0,
                refresh_rate_hz: 144,
                night_light_enabled: false,
            },
            sound: SoundConfig {
                master_volume: 75,
                boot_chime_enabled: true,
            },
            touchpad: TouchpadConfig {
                tap_to_click: true,
                natural_scrolling: true,
                sensitivity: 1.0,
            },
            keyboard: KeyboardConfig {
                layout: "us".to_string(),
                repeat_delay_ms: 300,
                repeat_interval_ms: 30,
            },
            accessibility: AccessibilityConfig {
                high_contrast: false,
                large_text: false,
            },
            appearance: AppearanceConfig {
                dark_mode: true,
                accent_color_hex: "#FF5500".to_string(), // Space Orange
            },
        }
    }
}
