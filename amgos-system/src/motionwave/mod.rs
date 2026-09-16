//! MotionWave Input Subsystem (Process 1)
//!
//! Owns input hardware topology, device capability probing, and configuration state
//! for touchpads, keyboards, and buttons.
//! Detects I2C multitouch hardware (e.g. ELAN0788 Touchpad), AT keyboards, and lid/power switches.
//! Process 2 receives input device updates over e-bus without touching raw evdev nodes.

use amgos_protocol::ebus::{InputDeviceInfo, InputDeviceType, KeyboardConfig, TouchpadConfig};
use std::fs;
use std::io;
use std::sync::{Arc, Mutex};

pub const INPUT_DEVICES_PROC_PATH: &str = "/proc/bus/input/devices";

pub struct MotionWaveController {
    touchpad_config: Arc<Mutex<TouchpadConfig>>,
    keyboard_config: Arc<Mutex<KeyboardConfig>>,
}

impl MotionWaveController {
    pub fn new() -> Self {
        Self {
            touchpad_config: Arc::new(Mutex::new(TouchpadConfig::default())),
            keyboard_config: Arc::new(Mutex::new(KeyboardConfig::default())),
        }
    }

    /// Enumerate all connected input devices from /proc/bus/input/devices
    pub fn enumerate_devices(&self) -> io::Result<Vec<InputDeviceInfo>> {
        let content = fs::read_to_string(INPUT_DEVICES_PROC_PATH)?;
        Ok(Self::parse_proc_devices(&content))
    }

    pub fn parse_proc_devices(content: &str) -> Vec<InputDeviceInfo> {
        let mut devices = Vec::new();
        let mut current_name = String::new();
        let mut current_vendor = 0u16;
        let mut current_product = 0u16;
        let mut current_handlers = String::new();
        let mut current_prop = String::new();
        let mut current_abs = false;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                // End of stanza
                if !current_name.is_empty() && !current_handlers.is_empty() {
                    if let Some(dev) = Self::build_device_info(
                        &current_name,
                        current_vendor,
                        current_product,
                        &current_handlers,
                        &current_prop,
                        current_abs,
                    ) {
                        devices.push(dev);
                    }
                }
                current_name.clear();
                current_vendor = 0;
                current_product = 0;
                current_handlers.clear();
                current_prop.clear();
                current_abs = false;
                continue;
            }

            if let Some(stripped) = line.strip_prefix("I: ") {
                for token in stripped.split_whitespace() {
                    if let Some((k, v)) = token.split_once('=') {
                        match k {
                            "Vendor" => current_vendor = u16::from_str_radix(v, 16).unwrap_or(0),
                            "Product" => current_product = u16::from_str_radix(v, 16).unwrap_or(0),
                            _ => {}
                        }
                    }
                }
            } else if let Some(stripped) = line.strip_prefix("N: Name=") {
                current_name = stripped.trim_matches('"').to_string();
            } else if let Some(stripped) = line.strip_prefix("H: Handlers=") {
                current_handlers = stripped.to_string();
            } else if let Some(stripped) = line.strip_prefix("B: PROP=") {
                current_prop = stripped.to_string();
            } else if line.starts_with("B: ABS=") {
                current_abs = true;
            }
        }

        // Flush trailing entry
        if !current_name.is_empty() && !current_handlers.is_empty() {
            if let Some(dev) = Self::build_device_info(
                &current_name,
                current_vendor,
                current_product,
                &current_handlers,
                &current_prop,
                current_abs,
            ) {
                devices.push(dev);
            }
        }

        devices
    }

    fn build_device_info(
        name: &str,
        vendor_id: u16,
        product_id: u16,
        handlers: &str,
        prop: &str,
        has_abs: bool,
    ) -> Option<InputDeviceInfo> {
        let mut event_node = String::new();
        for h in handlers.split_whitespace() {
            if h.starts_with("event") {
                event_node = format!("/dev/input/{}", h);
                break;
            }
        }

        if event_node.is_empty() {
            return None;
        }

        let name_lower = name.to_lowercase();
        let (device_type, is_multitouch) = if name_lower.contains("touchpad")
            || (has_abs
                && (prop.contains('5')
                    || name_lower.contains("elan")
                    || name_lower.contains("synaptics")))
        {
            (InputDeviceType::Touchpad, true)
        } else if name_lower.contains("keyboard") || handlers.contains("kbd") {
            (InputDeviceType::Keyboard, false)
        } else if name_lower.contains("mouse") {
            (InputDeviceType::Mouse, false)
        } else if name_lower.contains("touchscreen") {
            (InputDeviceType::Touchscreen, true)
        } else if name_lower.contains("switch") || name_lower.contains("button") {
            (InputDeviceType::Switch, false)
        } else {
            (InputDeviceType::Keyboard, false)
        };

        Some(InputDeviceInfo {
            sysfs_name: name.to_string(),
            event_node,
            device_type,
            vendor_id,
            product_id,
            is_multitouch,
        })
    }

    pub fn get_touchpad_config(&self) -> TouchpadConfig {
        self.touchpad_config.lock().unwrap().clone()
    }

    pub fn set_touchpad_config(&self, config: TouchpadConfig) {
        let mut cfg = self.touchpad_config.lock().unwrap();
        *cfg = config;
    }

    pub fn get_keyboard_config(&self) -> KeyboardConfig {
        self.keyboard_config.lock().unwrap().clone()
    }

    pub fn set_keyboard_config(&self, config: KeyboardConfig) {
        let mut cfg = self.keyboard_config.lock().unwrap();
        *cfg = config;
    }
}

impl Default for MotionWaveController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_input_devices() {
        let sample = r#"
I: Bus=0018 Vendor=04f3 Product=321a Version=0100
N: Name="ELAN0788:00 04F3:321A Touchpad"
P: Phys=i2c-ELAN0788:00
S: Sysfs=/devices/pci0000:00/0000:00:15.0/i2c_designware.0/i2c-0/i2c-ELAN0788:00/0018:04F3:321A.0001/input/input9
U: Uniq=
H: Handlers=mouse1 event5 
B: PROP=5
B: EV=1b
B: KEY=e520 10000 0 0 0 0
B: ABS=2e0800000000003
B: MSC=20

I: Bus=0011 Vendor=0001 Product=0001 Version=ab83
N: Name="AT Translated Set 2 keyboard"
P: Phys=isa0060/serio0/input0
S: Sysfs=/devices/platform/i8042/serio0/input/input3
U: Uniq=
H: Handlers=sysrq kbd event3 leds 
B: PROP=0
B: EV=120013

I: Bus=0019 Vendor=0000 Product=0001 Version=0000
N: Name="Power Button"
P: Phys=PNP0C0C/button/input0
S: Sysfs=/devices/platform/PNP0C0C:00/input/input1
U: Uniq=
H: Handlers=kbd event1 
B: PROP=0
B: EV=3
"#;

        let devices = MotionWaveController::parse_proc_devices(sample);
        assert_eq!(devices.len(), 3);

        let touchpad = devices
            .iter()
            .find(|d| d.device_type == InputDeviceType::Touchpad)
            .expect("Touchpad found");
        assert_eq!(touchpad.event_node, "/dev/input/event5");
        assert_eq!(touchpad.vendor_id, 0x04f3);
        assert_eq!(touchpad.product_id, 0x321a);
        assert!(touchpad.is_multitouch);

        let keyboard = devices
            .iter()
            .find(|d| {
                d.device_type == InputDeviceType::Keyboard && d.sysfs_name.contains("keyboard")
            })
            .expect("Keyboard found");
        assert_eq!(keyboard.event_node, "/dev/input/event3");
        assert_eq!(keyboard.vendor_id, 0x0001);
    }

    #[test]
    fn test_motionwave_config_state() {
        let mw = MotionWaveController::new();
        let default_tp = mw.get_touchpad_config();
        assert!(default_tp.tap_to_click);
        assert!(default_tp.natural_scrolling);

        let mut custom_tp = default_tp;
        custom_tp.pointer_speed = 0.75;
        mw.set_touchpad_config(custom_tp);
        assert!((mw.get_touchpad_config().pointer_speed - 0.75).abs() < 0.001);
    }
}
