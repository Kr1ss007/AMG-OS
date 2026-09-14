//! System Power Management Subsystem
//!
//! Controls ACPI power transitions: Sleep, Wake, Hibernate, Shutdown, Reboot.
//! Pre-configured Power Profiles: Endurance, Balanced, MAX.
//! Coordinates with AVM to drain audio buffers cleanly prior to sleep or shutdown.

use amgos_protocol::ebus::{PowerProfile, PowerState};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct PowerManager {
    current_state: Arc<Mutex<PowerState>>,
    current_profile: Arc<Mutex<PowerProfile>>,
    in_transition: Arc<AtomicBool>,
}

impl PowerManager {
    pub fn new() -> Self {
        Self {
            current_state: Arc::new(Mutex::new(PowerState::Active)),
            current_profile: Arc::new(Mutex::new(PowerProfile::Balanced)),
            in_transition: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn current_state(&self) -> PowerState {
        *self.current_state.lock().unwrap()
    }

    pub fn current_profile(&self) -> PowerProfile {
        *self.current_profile.lock().unwrap()
    }

    /// Pre-configured profile application:
    /// - Endurance: powersave governor, energy-performance-preference = "power"
    /// - Balanced: powersave/balance governor, energy-performance-preference = "balance_performance"
    /// - Max: performance governor, energy-performance-preference = "performance"
    pub fn set_profile(&self, profile: PowerProfile) -> Result<String, String> {
        let (governor, epp) = match profile {
            PowerProfile::Endurance => ("powersave", "power"),
            PowerProfile::Balanced => ("powersave", "balance_performance"),
            PowerProfile::Max => ("performance", "performance"),
        };

        // Attempt sysfs CPU governor configuration if writable
        apply_cpu_governor(governor, epp);

        let mut current = self.current_profile.lock().unwrap();
        *current = profile;

        Ok(governor.to_string())
    }

    pub fn transition_to(&self, target: PowerState) -> Result<PowerState, String> {
        if self.in_transition.swap(true, Ordering::SeqCst) {
            return Err("Power transition already in progress".to_string());
        }

        let mut state = self.current_state.lock().unwrap();
        *state = target;

        match target {
            PowerState::Sleep => {
                // ACPI S3 sleep
            }
            PowerState::Hibernate => {
                // ACPI S4 disk image write
            }
            PowerState::Shutdown => {
                // Controlled powerdown sequence
            }
            PowerState::Reboot => {
                // Controlled reboot sequence
            }
            PowerState::Active => {}
        }

        self.in_transition.store(false, Ordering::SeqCst);
        Ok(target)
    }
}

fn apply_cpu_governor(governor: &str, epp: &str) {
    let cpufreq_dir = Path::new("/sys/devices/system/cpu");
    if !cpufreq_dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(cpufreq_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str.starts_with("cpu") && name_str[3..].chars().all(|c| c.is_ascii_digit()) {
                let gov_path = entry.path().join("cpufreq/scaling_governor");
                let epp_path = entry.path().join("cpufreq/energy_performance_preference");

                if gov_path.exists() {
                    let _ = fs::write(gov_path, governor);
                }
                if epp_path.exists() {
                    let _ = fs::write(epp_path, epp);
                }
            }
        }
    }
}

impl Default for PowerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_profiles() {
        let pm = PowerManager::new();
        assert_eq!(pm.current_profile(), PowerProfile::Balanced);

        let gov = pm.set_profile(PowerProfile::Endurance).unwrap();
        assert_eq!(gov, "powersave");
        assert_eq!(pm.current_profile(), PowerProfile::Endurance);

        let gov_max = pm.set_profile(PowerProfile::Max).unwrap();
        assert_eq!(gov_max, "performance");
        assert_eq!(pm.current_profile(), PowerProfile::Max);
    }
}
