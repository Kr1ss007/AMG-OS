//! Audio & AVM Tray Indicator Subsystem
//!
//! Subscribes to Audio-Video Manager (AVM) events from Process 1.
//! Tracks volume levels (0-100%), mute status, active sinks, and ducking states.
//! Provides slider controls and quick mute toggle in the Top Panel system tray.

use amgos_protocol::ebus::SystemEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioTrayWidget {
    pub volume_pct: u8,
    pub is_muted: bool,
    pub is_hardware_ready: bool,
    pub active_sink_name: String,
    pub is_ducked: bool,
    pub popover_visible: bool,
}

impl AudioTrayWidget {
    pub fn new() -> Self {
        Self {
            volume_pct: 75,
            is_muted: false,
            is_hardware_ready: false,
            active_sink_name: "Internal Speakers (HDA DSP)".to_string(),
            is_ducked: false,
            popover_visible: false,
        }
    }

    /// Process e-bus SystemEvents related to audio and AVM
    pub fn handle_event(&mut self, event: &SystemEvent) {
        match event {
            SystemEvent::AudioReady { .. } => {
                self.is_hardware_ready = true;
            }
            SystemEvent::BootChimeTrigger { .. } => {
                // Boot chime is playing (uninterruptible)
                self.is_ducked = true;
            }
            _ => {}
        }
    }

    /// Set volume percentage directly (clamped 0 to 100)
    pub fn set_volume(&mut self, vol: u8) {
        self.volume_pct = vol.min(100);
        if self.volume_pct > 0 && self.is_muted {
            self.is_muted = false;
        }
    }

    /// Step volume up by 5%
    pub fn step_up(&mut self) {
        self.set_volume(self.volume_pct.saturating_add(5));
    }

    /// Step volume down by 5%
    pub fn step_down(&mut self) {
        self.set_volume(self.volume_pct.saturating_sub(5));
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
    }

    /// Icon asset name for WhiteSur icon theme
    pub fn icon_name(&self) -> &'static str {
        if self.is_muted || self.volume_pct == 0 {
            "audio-volume-muted"
        } else if self.volume_pct < 33 {
            "audio-volume-low"
        } else if self.volume_pct < 66 {
            "audio-volume-medium"
        } else {
            "audio-volume-high"
        }
    }

    pub fn tooltip_text(&self) -> String {
        if self.is_muted {
            format!("Audio: Muted ({})", self.active_sink_name)
        } else {
            format!("Audio: {}% ({})", self.volume_pct, self.active_sink_name)
        }
    }

    pub fn toggle_popover(&mut self) {
        self.popover_visible = !self.popover_visible;
    }
}

impl Default for AudioTrayWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_tray_volume_and_icons() {
        let mut widget = AudioTrayWidget::new();
        assert_eq!(widget.volume_pct, 75);
        assert_eq!(widget.icon_name(), "audio-volume-high");

        widget.set_volume(25);
        assert_eq!(widget.icon_name(), "audio-volume-low");

        widget.set_volume(50);
        assert_eq!(widget.icon_name(), "audio-volume-medium");

        widget.toggle_mute();
        assert!(widget.is_muted);
        assert_eq!(widget.icon_name(), "audio-volume-muted");

        // Stepping up unmutes
        widget.step_up();
        assert_eq!(widget.volume_pct, 55);
        assert!(!widget.is_muted);
    }
}
