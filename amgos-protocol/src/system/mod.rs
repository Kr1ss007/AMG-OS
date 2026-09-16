//! Messages published by Process 1 (System Session) across EventBus (e-bus)
//!
//! Enforces the two-process architecture boundary:
//! Process 1 owns hardware, audio, power, network credentials, Astrophage logs,
//! Filer indexing backend, Bluetooth adapter, and A/B update state.
//! Process 2 subscribes to these versioned binary events.

use serde::{Deserialize, Serialize};

/// Power states supported across AMGOS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    Active,
    Sleep,
    Hibernate,
    Shutdown,
    Reboot,
}

/// Pre-configured system power profile (Endurance, Balanced, MAX)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerProfile {
    Endurance,
    Balanced,
    Max,
}

/// Installation progress stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallStage {
    Downloading,
    Inspecting,
    Staging,
    Installing,
    Completed,
    Failed,
}

/// Astrophage rolling log entry payload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AstrophageRecord {
    pub timestamp_ns: u64,
    pub level: AstrophageLevel,
    pub subsystem: String,
    pub message: String,
    pub metric_key: Option<String>,
    pub metric_value: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstrophageLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Pathfinder query result item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathfinderItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: PathfinderCategory,
    pub score: f32,
    pub action_uri: String,
    pub is_installed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathfinderCategory {
    Application,
    File,
    Setting,
    InstallableWeb,
}

/// File entry for directory listings over e-bus
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolFileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: u64,
    pub modified_timestamp_secs: u64,
}

/// Package inspection report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionReport {
    pub package_name: String,
    pub version: String,
    pub architecture: String,
    pub sha256_digest: String,
    pub package_format: String, // "deb", "flatpak", "appimage", "snap"
    pub declared_permissions: Vec<String>,
    pub installed_size_bytes: u64,
    pub is_signed: bool,
    pub security_notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputDeviceType {
    Keyboard,
    Touchpad,
    Mouse,
    Touchscreen,
    Switch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputDeviceInfo {
    pub sysfs_name: String,
    pub event_node: String,
    pub device_type: InputDeviceType,
    pub vendor_id: u16,
    pub product_id: u16,
    pub is_multitouch: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TouchpadConfig {
    pub tap_to_click: bool,
    pub natural_scrolling: bool,
    pub pointer_speed: f32, // -1.0 to 1.0
    pub palm_rejection: bool,
    pub two_finger_scroll: bool,
}

impl Default for TouchpadConfig {
    fn default() -> Self {
        Self {
            tap_to_click: true,
            natural_scrolling: true,
            pointer_speed: 0.0,
            palm_rejection: true,
            two_finger_scroll: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardConfig {
    pub repeat_rate_hz: u32,
    pub repeat_delay_ms: u32,
    pub layout: String,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            repeat_rate_hz: 30,
            repeat_delay_ms: 250,
            layout: "us".to_string(),
        }
    }
}

/// WiFi access point information (no credentials — safe to send to Process 2)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiAccessPoint {
    pub ssid: String,
    pub signal_strength_pct: u8,
    pub is_secured: bool,
}

/// Bluetooth device information (no raw HCI/passkeys — safe for Process 2)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothDeviceInfo {
    pub address: String,
    pub name: Option<String>,
    pub device_type: String, // "audio", "input", "phone", "generic"
    pub is_paired: bool,
    pub is_connected: bool,
    pub is_trusted: bool,
    pub signal_rssi: Option<i16>,
}

/// A/B partition OTA update status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbSlotStatus {
    pub active_slot: String,
    pub standby_slot: String,
    pub boot_successful: bool,
    pub update_in_progress: bool,
    pub update_stage: String,
    pub progress_pct: f32,
}

/// Status of a DownloadManager job (mirrored from filer-core)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadJobStatus {
    Queued,
    Connecting,
    Downloading,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

/// Events emitted by Process 1 (System Session) across e-bus
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemEvent {
    /// Emitted once core hardware detection and drivers are operational
    HardwareReady {
        timestamp_ns: u64,
        cpu_model: String,
        cpu_cores: usize,
        total_memory_bytes: u64,
        discrete_gpu: String,
    },

    /// Emitted when Audio-Video Manager (AVM) is initialized
    AudioReady {
        timestamp_ns: u64,
        sample_rate: u32,
        channels: u16,
    },

    /// Emitted by AVM chime sequencer when audio hardware is synchronized
    BootChimeTrigger {
        timestamp_ns: u64,
        note: String,      // "F3"
        frequency_hz: f32, // 174.61 Hz
        duration_ms: u32,
    },

    /// Emitted when network state changes
    NetworkStateChanged {
        connected: bool,
        interface_name: String,
        ssid: Option<String>,
        ip_address: Option<String>,
    },

    /// Emitted on system power state transitions
    PowerStateTransition {
        target: PowerState,
        initiated_by_system: bool,
    },

    /// Emitted when power profile is switched (Endurance, Balanced, MAX)
    PowerProfileChanged {
        profile: PowerProfile,
        active_governor: String,
    },

    /// Emitted when display hardware configuration is changed
    DisplayConfigChanged {
        connector: String,
        width: u32,
        height: u32,
        refresh_rate_mhz: u32,
        scale_factor: f32,
    },

    /// Continuous rolling diagnostic events from Process 1
    AstrophageLog(AstrophageRecord),

    /// Snapshot response of rolling Astrophage buffer
    AstrophageBufferSnapshot { entries: Vec<AstrophageRecord> },

    /// Response to a Pathfinder search query
    PathfinderResults {
        query_id: u64,
        results: Vec<PathfinderItem>,
    },

    /// Directory listing response
    DirectoryListing {
        request_id: u64,
        path: String,
        entries: Vec<ProtocolFileEntry>,
    },

    /// Inspection report for an installable package
    PackageInspected {
        package_path: String,
        report: InspectionReport,
    },

    /// Real-time progress updates for package installation
    PackageProgress {
        package_path: String,
        stage: InstallStage,
        percent: f32,
    },

    /// Final outcome of package installation
    PackageInstallFinished {
        package_path: String,
        app_id: String,
        success: bool,
        error_message: Option<String>,
    },

    /// Final outcome of package uninstallation (symmetric with install)
    PackageUninstallFinished {
        app_id: String,
        success: bool,
        error_message: Option<String>,
    },

    /// Emitted when input hardware topology changes
    InputDevicesChanged { devices: Vec<InputDeviceInfo> },

    /// Emitted when touchpad configuration is updated
    TouchpadConfigChanged(TouchpadConfig),

    /// Emitted when keyboard configuration is updated
    KeyboardConfigChanged(KeyboardConfig),

    /// Emitted by eo-bus bridge when a foreign application updates its top-bar menu
    ForeignAppMenuUpdated { app_id: String, menu_json: String },

    /// Emitted by eo-bus bridge when a desktop notification is dispatched by a third-party app
    NotificationDispatched {
        notification_id: u64,
        app_id: String,
        title: String,
        body: String,
        urgency: u8,
    },

    /// Response to a WiFi access point scan request
    WifiScanResults { access_points: Vec<WifiAccessPoint> },

    /// Download progress update from the DownloadManager
    DownloadProgress {
        job_id: u64,
        url: String,
        bytes_received: u64,
        total_bytes: u64, // 0 if unknown (streaming)
        percent: f32,     // 0.0 if total unknown
        status: DownloadJobStatus,
    },

    /// Emitted when Process 1 has applied the signed first-boot configuration
    /// from the Setup Wizard. Signals Process 2 to dismiss wizard and show desktop.
    FirstBootConfigApplied {
        user_name: String,
        locale: String,
        ui_scale_factor: f32,
    },

    /// Emitted when Bluetooth adapter status or discovered devices change
    BluetoothStatusChanged {
        powered: bool,
        discovering: bool,
        devices: Vec<BluetoothDeviceInfo>,
    },

    /// Emitted when A/B partition slot status or OTA update state changes
    AbSlotStatusChanged(AbSlotStatus),
}
