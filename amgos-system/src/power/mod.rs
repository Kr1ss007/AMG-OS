//! System Power Management Subsystem
//!
//! Controls ACPI power transitions: Sleep (S3), Hibernate (S4), Shutdown, Reboot.
//! Pre-configured Power Profiles: Endurance, Balanced, MAX.
//! Applies governor + energy_performance_preference via sysfs for each profile.
//! Coordinates with AVM (see main.rs) to signal audio drain before state transitions.
//!
//! Hardware target: Intel Core i5-13420H + NVIDIA RTX 3050 (discrete, no PRIME).
//! ACPI transitions write to /sys/power/state as real sysfs writes.
//! Shutdown and reboot invoke systemctl as the init PID 1 interface (not shell scripts).

use amgos_protocol::ebus::{PowerProfile, PowerState};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct PowerManager {
    current_state: Arc<Mutex<PowerState>>,
    current_profile: Arc<Mutex<PowerProfile>>,
    in_transition: Arc<AtomicBool>,
}

impl PowerManager {
    pub fn new() -> Self {
        let pm = Self {
            current_state: Arc::new(Mutex::new(PowerState::Active)),
            current_profile: Arc::new(Mutex::new(PowerProfile::Balanced)),
            in_transition: Arc::new(AtomicBool::new(false)),
        };

        // Apply Balanced profile on init so sysfs is in a known state from boot
        let _ = pm.set_profile(PowerProfile::Balanced);
        pm
    }

    pub fn current_state(&self) -> PowerState {
        *self.current_state.lock().unwrap()
    }

    pub fn current_profile(&self) -> PowerProfile {
        *self.current_profile.lock().unwrap()
    }

    /// Apply a pre-configured system power profile by writing to sysfs.
    ///
    /// Profile mappings (per AMGOS spec):
    ///   Endurance  → powersave governor, EPP = "power"
    ///   Balanced   → powersave governor, EPP = "balance_performance"
    ///   Max        → performance governor, EPP = "performance"
    ///
    /// Returns the active scaling_governor string on success.
    pub fn set_profile(&self, profile: PowerProfile) -> Result<String, String> {
        let (governor, epp) = match profile {
            PowerProfile::Endurance => ("powersave", "power"),
            PowerProfile::Balanced => ("powersave", "balance_performance"),
            PowerProfile::Max => ("performance", "performance"),
        };

        apply_cpu_governor(governor, epp);
        apply_platform_profile(profile);

        let mut current = self.current_profile.lock().unwrap();
        *current = profile;

        Ok(governor.to_string())
    }

    /// Initiate a system power state transition.
    ///
    /// Sleep     → writes "mem" to /sys/power/state  (ACPI S3)
    /// Hibernate → writes "disk" to /sys/power/state (ACPI S4)
    /// Shutdown  → invokes `systemctl poweroff`       (init-managed)
    /// Reboot    → invokes `systemctl reboot`         (init-managed)
    /// Active    → no-op (already active)
    pub fn transition_to(&self, target: PowerState) -> Result<PowerState, String> {
        if self.in_transition.swap(true, Ordering::SeqCst) {
            return Err("Power transition already in progress".to_string());
        }

        let result = self.perform_transition(target);

        self.in_transition.store(false, Ordering::SeqCst);
        result
    }

    fn perform_transition(&self, target: PowerState) -> Result<PowerState, String> {
        match target {
            PowerState::Sleep => {
                // Flush all VFS dirty pages before suspending to prevent data loss
                sync_filesystem();

                // Write ACPI S3 sleep state
                match write_power_state("mem") {
                    Ok(()) => {
                        // Execution resumes here after wake
                        let mut state = self.current_state.lock().unwrap();
                        *state = PowerState::Active;
                        Ok(PowerState::Active)
                    }
                    Err(e) => Err(format!("Failed to write sleep state: {e}")),
                }
            }

            PowerState::Hibernate => {
                // Hibernate requires swap space to be available (checked at runtime)
                if !has_swap() {
                    return Err("Hibernate unavailable: no swap space configured".to_string());
                }

                sync_filesystem();

                match write_power_state("disk") {
                    Ok(()) => {
                        let mut state = self.current_state.lock().unwrap();
                        *state = PowerState::Active;
                        Ok(PowerState::Active)
                    }
                    Err(e) => Err(format!("Failed to write hibernate state: {e}")),
                }
            }

            PowerState::Shutdown => {
                let mut state = self.current_state.lock().unwrap();
                *state = PowerState::Shutdown;
                drop(state);

                sync_filesystem();

                // Delegate to systemd (PID 1) for clean process tree teardown
                invoke_systemctl("poweroff")
                    .map_err(|e| format!("systemctl poweroff failed: {e}"))?;
                Ok(PowerState::Shutdown)
            }

            PowerState::Reboot => {
                let mut state = self.current_state.lock().unwrap();
                *state = PowerState::Reboot;
                drop(state);

                sync_filesystem();

                invoke_systemctl("reboot").map_err(|e| format!("systemctl reboot failed: {e}"))?;
                Ok(PowerState::Reboot)
            }

            PowerState::Active => {
                // Already active — no-op
                Ok(PowerState::Active)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sysfs / procfs utilities
// ---------------------------------------------------------------------------

/// Write "powersave"/"performance" to scaling_governor and EPP for every CPU core
fn apply_cpu_governor(governor: &str, epp: &str) {
    let cpufreq_dir = Path::new("/sys/devices/system/cpu");
    if !cpufreq_dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(cpufreq_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            // Only consider logical CPUs: cpu0, cpu1, ... (not cpufreq/, cpuidle/)
            if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                let gov_path = entry.path().join("cpufreq/scaling_governor");
                let epp_path = entry.path().join("cpufreq/energy_performance_preference");

                if gov_path.exists() {
                    let _ = fs::write(&gov_path, governor);
                }
                if epp_path.exists() {
                    let _ = fs::write(&epp_path, epp);
                }
            }
        }
    }
}

/// Write to the ACPI platform_profile sysfs node (available on modern laptops)
fn apply_platform_profile(profile: PowerProfile) {
    let profile_str = match profile {
        PowerProfile::Endurance => "low-power",
        PowerProfile::Balanced => "balanced",
        PowerProfile::Max => "performance",
    };

    let path = Path::new("/sys/firmware/acpi/platform_profile");
    if path.exists() {
        let _ = fs::write(path, profile_str);
    }
}

/// Write to /sys/power/state to initiate ACPI sleep or hibernate
fn write_power_state(state: &str) -> Result<(), std::io::Error> {
    let path = Path::new("/sys/power/state");
    // Verify the requested state is supported
    let supported = fs::read_to_string("/sys/power/state").unwrap_or_default();
    if !supported.split_whitespace().any(|s| s == state) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("ACPI power state '{state}' not listed in /sys/power/state"),
        ));
    }
    fs::write(path, state)
}

/// Issue sync(2) equivalent: write to /proc/sysrq-trigger "s" for sync,
/// or use the `sync` binary to ensure VFS dirty pages are flushed before sleep.
fn sync_filesystem() {
    // Attempt via sysrq (requires kernel sysrq support)
    let sysrq_path = Path::new("/proc/sysrq-trigger");
    if sysrq_path.exists() {
        let _ = fs::write(sysrq_path, "s");
    }
    // Also invoke sync(1) for belt-and-suspenders reliability
    let _ = Command::new("sync").status();
}

/// Check if any swap partition is active in /proc/swaps
fn has_swap() -> bool {
    fs::read_to_string("/proc/swaps")
        .map(|s| {
            s.lines()
                .skip(1) // Header line
                .any(|l| !l.trim().is_empty())
        })
        .unwrap_or(false)
}

/// Invoke systemctl for init-managed power transitions (shutdown/reboot).
/// Spawns as a blocking call so Process 1 does not continue its main loop.
fn invoke_systemctl(action: &str) -> std::io::Result<()> {
    Command::new("systemctl").arg(action).status().map(|_| ())
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
    fn test_power_profiles_governor_mapping() {
        let pm = PowerManager::new();
        assert_eq!(pm.current_profile(), PowerProfile::Balanced);

        let gov = pm.set_profile(PowerProfile::Endurance).unwrap();
        assert_eq!(gov, "powersave");
        assert_eq!(pm.current_profile(), PowerProfile::Endurance);

        let gov_max = pm.set_profile(PowerProfile::Max).unwrap();
        assert_eq!(gov_max, "performance");
        assert_eq!(pm.current_profile(), PowerProfile::Max);

        // Restore balanced
        let gov_bal = pm.set_profile(PowerProfile::Balanced).unwrap();
        assert_eq!(gov_bal, "powersave");
    }

    #[test]
    fn test_power_state_active_noop() {
        let pm = PowerManager::new();
        let result = pm.transition_to(PowerState::Active);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PowerState::Active);
    }

    #[test]
    fn test_swap_detection() {
        // /proc/swaps always exists on Linux; result depends on host config
        let _ = has_swap(); // just ensure no panic
    }

    #[test]
    fn test_no_double_transition() {
        let pm = PowerManager::new();
        // Set in_transition manually and verify the guard fires
        pm.in_transition.store(true, Ordering::SeqCst);
        let result = pm.transition_to(PowerState::Active);
        // Should be rejected because in_transition is set
        assert!(result.is_err());
        pm.in_transition.store(false, Ordering::SeqCst);
    }
}
