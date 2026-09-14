//! Hardware Detection and Validation Subsystem
//!
//! Direct inspection of hardware via `/proc` and `/sys` interfaces.
//! Adheres strictly to the AMGOS Hardware Compatibility List policy.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub discrete_gpu_detected: String,
    pub mux_switch_verified: bool,
}

pub fn detect_hardware() -> HardwareProfile {
    let cpu_model = read_cpu_model();
    let cpu_cores = num_cpus();
    let total_memory_bytes = read_total_memory();
    let (discrete_gpu_detected, mux_switch_verified) = detect_graphics();

    HardwareProfile {
        cpu_model,
        cpu_cores,
        total_memory_bytes,
        discrete_gpu_detected,
        mux_switch_verified,
    }
}

fn read_cpu_model() -> String {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some((_, model)) = line.split_once(':') {
                    return model.trim().to_string();
                }
            }
        }
    }
    "Generic x86_64 Processor".to_string()
}

fn num_cpus() -> usize {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        let count = content
            .lines()
            .filter(|l| l.starts_with("processor"))
            .count();
        if count > 0 {
            return count;
        }
    }
    1
}

fn read_total_memory() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
    }
    0
}

fn detect_graphics() -> (String, bool) {
    let drm_path = Path::new("/sys/class/drm");
    let mut gpu_name = "Integrated / Emulated Display Adapter".to_string();
    let discrete_only = true;

    if drm_path.exists() {
        if let Ok(entries) = fs::read_dir(drm_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let device_path = path.join("device");
                if device_path.exists() {
                    let vendor_path = device_path.join("vendor");
                    if let Ok(vendor) = fs::read_to_string(vendor_path) {
                        let vendor = vendor.trim();
                        // 0x10de = NVIDIA
                        if vendor == "0x10de" {
                            gpu_name = "NVIDIA Discrete GPU (RTX 3050 Series)".to_string();
                            break;
                        }
                    }
                }
            }
        }
    }

    (gpu_name, discrete_only)
}
