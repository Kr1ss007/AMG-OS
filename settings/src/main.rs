//! Settings: AMGOS Central System Settings Application
//!
//! Owns:
//! - Single unified user-facing configuration interface
//! - Display, Sound, Network, Touchpad, Keyboard, Accessibility, Privacy, Appearance
//! - OTA updates via A/B partition swap
//!
//! Rule: Writes exclusively to signed configuration state owned by Process 1.
//! Never writes files directly to disk.

pub mod config;
pub mod ota;

use config::SystemConfiguration;
use ota::OtaUpdateState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[settings] Initializing AMGOS Settings application...");

    let config = SystemConfiguration::default();
    let ota = OtaUpdateState::new();

    println!(
        "[settings] Platform: AMGOS v{} ({})",
        ota.current_version, ota.current_codename
    );
    println!(
        "[settings] Active display configuration: {}Hz, Scale: {:.1}",
        config.display.refresh_rate_hz, config.display.scale_factor
    );
    println!(
        "[settings] Audio configuration: Volume {}%, Boot Chime: {}",
        config.sound.master_volume, config.sound.boot_chime_enabled
    );
    println!(
        "[settings] Appearance: Dark Mode: {}, Accent: {}",
        config.appearance.dark_mode, config.appearance.accent_color_hex
    );

    println!("[settings] Ready for presentation.");
    Ok(())
}
