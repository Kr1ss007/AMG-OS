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

    /// Synthesize real 48,000 Hz 16-bit stereo PCM audio for the F3 piano boot chime.
    /// Incorporates acoustic piano harmonics (F3, F4, C5, F5, A5), ADSR hammer envelope,
    /// and simulated room reverb.
    #[allow(clippy::needless_range_loop)]
    pub fn synthesize_boot_chime_pcm(&self) -> Vec<u8> {
        let total_samples = (AUDIO_SAMPLE_RATE as f32 * (CHIME_DURATION_MS as f32 / 1000.0)) as usize;
        let mut pcm_bytes = Vec::with_capacity(total_samples * (AUDIO_CHANNELS as usize) * 2);

        let f0 = CHIME_FREQUENCY_HZ;
        let two_pi = 2.0 * std::f32::consts::PI;

        // Precompute raw harmonic waveform
        let mut raw_samples = vec![0.0f32; total_samples];
        for i in 0..total_samples {
            let t = i as f32 / AUDIO_SAMPLE_RATE as f32;

            // Attack envelope (6ms rise)
            let attack = (t / 0.006).min(1.0);

            // Piano harmonics with frequency-dependent decay rates
            let h1 = (two_pi * f0 * t).sin() * 1.00 * (-t / 0.65).exp();
            let h2 = (two_pi * 2.0 * f0 * t).sin() * 0.55 * (-t / 0.45).exp();
            let h3 = (two_pi * 3.0 * f0 * t).sin() * 0.28 * (-t / 0.35).exp(); // Fifth (C5)
            let h4 = (two_pi * 4.0 * f0 * t).sin() * 0.15 * (-t / 0.25).exp();
            let h5 = (two_pi * 5.0 * f0 * t).sin() * 0.08 * (-t / 0.18).exp(); // Third (A5)

            let body_resonance = (two_pi * 85.0 * t).sin() * 0.05 * (-t / 0.30).exp();

            let sample = attack * (h1 + h2 + h3 + h4 + h5 + body_resonance);
            raw_samples[i] = sample;
        }

        // Apply reverb reflection taps (28ms, 45ms, 72ms)
        let tap1 = (AUDIO_SAMPLE_RATE as f32 * 0.028) as usize;
        let tap2 = (AUDIO_SAMPLE_RATE as f32 * 0.045) as usize;
        let tap3 = (AUDIO_SAMPLE_RATE as f32 * 0.072) as usize;

        for i in 0..total_samples {
            let mut wet = raw_samples[i];
            if i >= tap1 {
                wet += raw_samples[i - tap1] * 0.18;
            }
            if i >= tap2 {
                wet += raw_samples[i - tap2] * 0.12;
            }
            if i >= tap3 {
                wet += raw_samples[i - tap3] * 0.08;
            }

            // Scale to 16-bit range with headroom
            let clamped = (wet * 0.65).clamp(-1.0, 1.0);
            let sample_i16 = (clamped * 32767.0) as i16;
            let sample_bytes = sample_i16.to_le_bytes();

            // Stereo: Left & Right
            pcm_bytes.extend_from_slice(&sample_bytes);
            pcm_bytes.extend_from_slice(&sample_bytes);
        }

        pcm_bytes
    }

    /// Generates standard 44-byte RIFF/WAVE header for the PCM buffer
    pub fn generate_riff_wav_header(
        pcm_len: usize,
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
    ) -> [u8; 44] {
        let mut header = [0u8; 44];
        let byte_rate = sample_rate * (channels as u32) * (bits_per_sample as u32) / 8;
        let block_align = channels * bits_per_sample / 8;
        let chunk_size = 36 + (pcm_len as u32);

        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&chunk_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");

        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
        header[20..22].copy_from_slice(&1u16.to_le_bytes()); // AudioFormat (1 = PCM)
        header[22..24].copy_from_slice(&channels.to_le_bytes());
        header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        header[32..34].copy_from_slice(&block_align.to_le_bytes());
        header[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());

        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&(pcm_len as u32).to_le_bytes());

        header
    }

    /// Render synthesized F3 chime to a valid RIFF WAV audio file
    pub fn export_boot_chime_wav<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<std::path::PathBuf> {
        let pcm = self.synthesize_boot_chime_pcm();
        let header = Self::generate_riff_wav_header(pcm.len(), AUDIO_SAMPLE_RATE, AUDIO_CHANNELS, 16);

        let dest = path.as_ref().to_path_buf();
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut file = std::fs::File::create(&dest)?;
        use std::io::Write;
        file.write_all(&header)?;
        file.write_all(&pcm)?;
        file.sync_all()?;

        Ok(dest)
    }

    /// Play the boot chime directly on host audio hardware
    pub fn play_boot_chime_hardware(&self) -> std::io::Result<()> {
        let tmp_wav = std::env::temp_dir().join("amgos_boot_chime.wav");
        self.export_boot_chime_wav(&tmp_wav)?;

        // Try PipeWire pw-play first (standard on host)
        let pw_res = std::process::Command::new("pw-play")
            .arg(&tmp_wav)
            .spawn();

        if pw_res.is_ok() {
            return Ok(());
        }

        // Try ALSA aplay fallback
        let aplay_res = std::process::Command::new("aplay")
            .arg("-q")
            .arg(&tmp_wav)
            .spawn();

        if aplay_res.is_ok() {
            return Ok(());
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avm_chime_sequencer_and_synthesis() {
        let avm = AudioVideoManager::new();
        assert!(!avm.is_audio_ready());
        assert!(avm.evaluate_chime_sequencer().is_none());

        avm.mark_audio_hardware_ready();
        assert!(avm.is_audio_ready());

        let chime = avm.evaluate_chime_sequencer().expect("Chime should trigger on hardware ready");
        assert_eq!(chime.note, "F3");
        assert!((chime.frequency_hz - 174.614).abs() < 0.01);
        assert_eq!(chime.sample_rate, 48000);
        assert_eq!(chime.channels, 2);

        // Chime must not trigger a second time (fires exactly once)
        assert!(avm.evaluate_chime_sequencer().is_none());

        // Verify PCM audio synthesis
        let pcm = avm.synthesize_boot_chime_pcm();
        let expected_bytes = 48000 * 1500 / 1000 * 2 * 2; // samples * channels * bytes_per_sample
        assert_eq!(pcm.len(), expected_bytes);
    }

    #[test]
    fn test_avm_stream_ducking() {
        let avm = AudioVideoManager::new();
        assert_eq!(
            avm.calculate_ducking_factor(AudioStreamPriority::MediaMusicVideo, AudioStreamPriority::BootChime),
            0.0
        );
        assert_eq!(
            avm.calculate_ducking_factor(AudioStreamPriority::MediaMusicVideo, AudioStreamPriority::SystemSound),
            0.2
        );
        assert_eq!(
            avm.calculate_ducking_factor(AudioStreamPriority::SystemSound, AudioStreamPriority::AppAudio),
            1.0
        );
    }

    #[test]
    fn test_avm_export_boot_chime_wav() {
        let avm = AudioVideoManager::new();
        let tmp_wav = std::env::temp_dir().join("amgos_test_chime.wav");
        let path = avm.export_boot_chime_wav(&tmp_wav).expect("Should export valid WAV");
        assert!(path.exists());

        // Check WAV header
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");

        let _ = std::fs::remove_file(&path);
    }
}
