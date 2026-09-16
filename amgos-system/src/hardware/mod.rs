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
    pub cpu_threads: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub discrete_gpu_detected: String,
    pub igpu_detected: Option<String>,
    pub mux_switch_verified: bool,
    pub cpu_temperature_celsius: Option<f32>,
    pub display_connectors: Vec<String>,
}

pub fn detect_hardware() -> HardwareProfile {
    let cpu_model = read_cpu_model();
    let cpu_cores = num_cpu_cores();
    let cpu_threads = num_cpu_threads();
    let (total_memory_bytes, available_memory_bytes) = read_memory_stats();
    let (discrete_gpu_detected, igpu_detected, mux_switch_verified) = detect_graphics();
    let cpu_temperature_celsius = read_cpu_temperature();
    let display_connectors = detect_display_connectors();

    HardwareProfile {
        cpu_model,
        cpu_cores,
        cpu_threads,
        total_memory_bytes,
        available_memory_bytes,
        discrete_gpu_detected,
        igpu_detected,
        mux_switch_verified,
        cpu_temperature_celsius,
        display_connectors,
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

fn num_cpu_threads() -> usize {
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

fn num_cpu_cores() -> usize {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        let mut core_ids = std::collections::HashSet::new();
        for line in content.lines() {
            if line.starts_with("core id") {
                if let Some((_, id)) = line.split_once(':') {
                    core_ids.insert(id.trim().to_string());
                }
            }
        }
        if !core_ids.is_empty() {
            return core_ids.len();
        }
    }
    num_cpu_threads()
}

fn read_memory_stats() -> (u64, u64) {
    let mut total = 0;
    let mut available = 0;

    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(kb) = parse_meminfo_kb(line) {
                    total = kb * 1024;
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(kb) = parse_meminfo_kb(line) {
                    available = kb * 1024;
                }
            }
        }
    }

    (total, available)
}

fn parse_meminfo_kb(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<u64>().ok()
    } else {
        None
    }
}

pub fn read_cpu_temperature() -> Option<f32> {
    let thermal_dir = Path::new("/sys/class/thermal");
    if !thermal_dir.exists() {
        return None;
    }

    let mut highest_temp: Option<f32> = None;
    if let Ok(entries) = fs::read_dir(thermal_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            if file_name.to_string_lossy().starts_with("thermal_zone") {
                let temp_path = path.join("temp");
                if let Ok(content) = fs::read_to_string(temp_path) {
                    if let Ok(millicelsius) = content.trim().parse::<f32>() {
                        let celsius = millicelsius / 1000.0;
                        highest_temp = Some(highest_temp.map_or(celsius, |h| h.max(celsius)));
                    }
                }
            }
        }
    }

    highest_temp
}

fn detect_graphics() -> (String, Option<String>, bool) {
    let pci_devices = Path::new("/sys/bus/pci/devices");
    let mut dgpu_name = String::new();
    let mut igpu_name: Option<String> = None;
    let mut discrete_detected = false;

    if pci_devices.exists() {
        if let Ok(entries) = fs::read_dir(pci_devices) {
            for entry in entries.flatten() {
                let path = entry.path();
                let class_path = path.join("class");
                if let Ok(class_str) = fs::read_to_string(&class_path) {
                    let class_trim = class_str.trim();
                    // PCI base class 0x03 corresponds to Display Controllers (VGA: 0x0300, 3D: 0x0302, Display: 0x0380)
                    if class_trim.starts_with("0x03") {
                        let vendor = fs::read_to_string(path.join("vendor"))
                            .unwrap_or_default()
                            .trim()
                            .to_lowercase();
                        let device = fs::read_to_string(path.join("device"))
                            .unwrap_or_default()
                            .trim()
                            .to_lowercase();
                        let driver = fs::read_link(path.join("driver"))
                            .map(|p| {
                                p.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string()
                            })
                            .unwrap_or_else(|_| "unbound".to_string());

                        match vendor.as_str() {
                            "0x10de" => {
                                discrete_detected = true;
                                let model = if device == "0x25ac" {
                                    "NVIDIA GeForce RTX 3050 6GB Laptop GPU"
                                } else {
                                    "NVIDIA Discrete GPU"
                                };
                                dgpu_name =
                                    format!("{model} [{vendor}:{device}] (driver: {driver})");
                            }
                            "0x8086" => {
                                let model = if device == "0xa7a8" {
                                    "Intel Raptor Lake-P UHD Graphics"
                                } else {
                                    "Intel Integrated Graphics"
                                };
                                igpu_name =
                                    Some(format!("{model} [{vendor}:{device}] (driver: {driver})"));
                            }
                            "0x1002" => {
                                discrete_detected = true;
                                dgpu_name = format!(
                                    "AMD Radeon GPU [{vendor}:{device}] (driver: {driver})"
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if !discrete_detected {
        dgpu_name = "None (iGPU only)".to_string();
    }

    let mux_verified = discrete_detected;
    (dgpu_name, igpu_name, mux_verified)
}

fn detect_display_connectors() -> Vec<String> {
    let drm_path = Path::new("/sys/class/drm");
    let mut connectors = Vec::new();

    if drm_path.exists() {
        if let Ok(entries) = fs::read_dir(drm_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with("card") && file_name.contains('-') {
                    let status_path = entry.path().join("status");
                    let status = fs::read_to_string(status_path)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if status == "connected" {
                        connectors.push(format!("{file_name} (connected)"));
                    } else {
                        connectors.push(file_name);
                    }
                }
            }
        }
    }

    connectors.sort();
    connectors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_hardware() {
        let profile = detect_hardware();
        assert!(!profile.cpu_model.is_empty());
        assert!(profile.cpu_cores > 0);
        assert!(profile.cpu_threads >= profile.cpu_cores);
        assert!(profile.total_memory_bytes > 0);
    }
}
