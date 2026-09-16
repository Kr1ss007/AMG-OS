//! Messages dispatched by Process 2 (Desktop Session) to Process 1 across EventBus (e-bus)
//!
//! Enforces the two-process architecture boundary:
//! Process 2 never accesses hardware, credentials, or raw filesystems directly.
//! All user-initiated actions, queries, configuration updates, and hardware controls
//! are dispatched as strongly typed `DesktopRequest` messages to Process 1.

use crate::system::{KeyboardConfig, PowerProfile, PowerState, TouchpadConfig};
use serde::{Deserialize, Serialize};

/// Requests dispatched by Process 2 (Desktop Session) to Process 1 across e-bus
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DesktopRequest {
    /// Query the Pathfinder index
    SearchPathfinder {
        query_id: u64,
        query: String,
        max_results: usize,
    },

    /// Request file directory listing
    ListDirectory { request_id: u64, path: String },

    /// Inspect a package file located at a path
    InspectPackage { package_path: String },

    /// Trigger installation of an inspected package into the app layer
    InstallPackage {
        package_path: String,
        confirmed_permissions: bool,
    },

    /// Trigger clean uninstallation of an app (zero residual state)
    UninstallPackage { app_id: String },

    /// Request a system power transition (sleep, shutdown, reboot)
    RequestPowerState(PowerState),

    /// Switch active power profile (Endurance, Balanced, MAX)
    SetPowerProfile(PowerProfile),

    /// Submit network credentials securely to Process 1 vault
    ConnectWifi { ssid: String, passphrase: String },

    /// Request diagnostic history from Astrophage
    GetAstrophageBuffer { max_entries: usize },

    /// Apply signed system configuration produced by Settings
    ApplySignedConfig {
        config_payload: Vec<u8>,
        hmac_signature: [u8; 32],
    },

    /// Request enumeration of input devices
    GetInputDevices,

    /// Update touchpad configuration in Process 1
    SetTouchpadConfig(TouchpadConfig),

    /// Update keyboard configuration in Process 1
    SetKeyboardConfig(KeyboardConfig),

    /// Dismiss notification in Process 1
    DismissNotification { notification_id: u64 },

    /// Request a WiFi access point scan (returns WifiScanResults event)
    ScanWifiAccessPoints,

    /// Request Process 1 to begin downloading a package from a URL into staging
    DownloadPackage { url: String },

    /// Cancel an in-flight package download
    CancelDownload { job_id: u64 },

    /// Submit completed Setup Wizard configuration (signed payload) to Process 1
    FirstBootWizardComplete {
        user_name: String,
        locale: String,
        ssid: Option<String>,
        ui_scale_factor: f32,
        telemetry_opt_in: bool,
        config_payload: Vec<u8>,
        hmac_signature: [u8; 32],
    },

    /// Toggle Bluetooth adapter power
    SetBluetoothPower(bool),

    /// Start Bluetooth discovery
    StartBluetoothScan,

    /// Stop Bluetooth discovery
    StopBluetoothScan,

    /// Pair or connect to Bluetooth device
    ConnectBluetoothDevice { address: String },

    /// Disconnect Bluetooth device
    DisconnectBluetoothDevice { address: String },

    /// Query current A/B partition status
    GetAbSlotStatus,

    /// Stage and apply OTA update from a squashfs image file
    ApplyOtaUpdate {
        image_path: String,
        expected_sha256: String,
    },
}
