//! Audio-Video Manager (AVM) Subsystem
//!
//! Owns all audio and video routing, PipeWire/WirePlumber policy,
//! stream priorities, ducking, and the hardware-ready boot chime sequencer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const CHIME_NOTE: &str = "F3";
pub const CHIME_FREQUENCY_HZ: f32 = 174.614; // F3 fundamental frequency
pub const CHIME_DURATION_MS: u32 = 1500;
pub const AUDIO_SAMPLE_RATE: u32 = 48000;
pub const AUDIO_CHANNELS: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AudioStreamPriority {
    AppAudio = 1,
    MediaMusicVideo = 2,
    SystemSound = 3,
    BootChime = 4, // Highest priority, cannot be ducked or interrupted
}

pub struct AudioVideoManager {
    audio_hardware_ready: Arc<AtomicBool>,
    chime_fired: Arc<AtomicBool>,
}

impl AudioVideoManager {
    pub fn new() -> Self {
        Self {
            audio_hardware_ready: Arc::new(AtomicBool::new(false)),
            chime_fired: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Called when PipeWire/ALSA confirms audio hardware is ready
    pub fn mark_audio_hardware_ready(&self) {
        self.audio_hardware_ready.store(true, Ordering::SeqCst);
    }

    pub fn is_audio_ready(&self) -> bool {
        self.audio_hardware_ready.load(Ordering::SeqCst)
    }

    /// Evaluates whether boot chime should fire.
    /// Fires exactly once upon hardware-ready confirmation.
    pub fn evaluate_chime_sequencer(&self) -> Option<ChimeEvent> {
        if self.audio_hardware_ready.load(Ordering::SeqCst)
            && !self.chime_fired.swap(true, Ordering::SeqCst)
        {
            Some(ChimeEvent {
                note: CHIME_NOTE.to_string(),
                frequency_hz: CHIME_FREQUENCY_HZ,
                duration_ms: CHIME_DURATION_MS,
                sample_rate: AUDIO_SAMPLE_RATE,
                channels: AUDIO_CHANNELS,
            })
        } else {
            None
        }
    }

    /// Calculate ducking volume factor based on active higher priority streams
    pub fn calculate_ducking_factor(
        &self,
        current_priority: AudioStreamPriority,
        active_priority: AudioStreamPriority,
    ) -> f32 {
        if active_priority > current_priority {
            match active_priority {
                AudioStreamPriority::BootChime => 0.0, // Complete mute during boot chime
                AudioStreamPriority::SystemSound => 0.2, // Duck music/video down to 20%
                _ => 1.0,
            }
        } else {
            1.0
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChimeEvent {
    pub note: String,
    pub frequency_hz: f32,
    pub duration_ms: u32,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for AudioVideoManager {
    fn default() -> Self {
        Self::new()
    }
}
