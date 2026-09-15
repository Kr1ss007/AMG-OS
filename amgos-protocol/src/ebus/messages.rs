//! AMGOS EventBus (e-bus) Typed Message Contracts
//!
//! Enforces the hard two-process boundary:
//! - Process 1 publishes `SystemEvent` (hardware state, network changes, AVM events,
//!   Astrophage log entries, installer status).
//! - Process 2 sends `DesktopRequest` (user-initiated actions, queries, power requests).

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
    AstrophageBufferSnapshot {
        entries: Vec<AstrophageRecord>,
    },

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
    InputDevicesChanged {
        devices: Vec<InputDeviceInfo>,
    },

    /// Emitted when touchpad configuration is updated
    TouchpadConfigChanged(TouchpadConfig),

    /// Emitted when keyboard configuration is updated
    KeyboardConfigChanged(KeyboardConfig),

    /// Emitted by eo-bus bridge when a foreign application updates its top-bar menu
    ForeignAppMenuUpdated {
        app_id: String,
        menu_json: String,
    },

    /// Emitted by eo-bus bridge when a desktop notification is dispatched by a third-party app
    NotificationDispatched {
        notification_id: u64,
        app_id: String,
        title: String,
        body: String,
        urgency: u8,
    },

    /// Response to a WiFi access point scan request
    WifiScanResults {
        access_points: Vec<WifiAccessPoint>,
    },

    /// Download progress update from the DownloadManager
    DownloadProgress {
        job_id: u64,
        url: String,
        bytes_received: u64,
        total_bytes: u64,   // 0 if unknown (streaming)
        percent: f32,       // 0.0 if total unknown
        status: DownloadJobStatus,
    },

    /// Emitted when Process 1 has applied the signed first-boot configuration
    /// from the Setup Wizard. Signals Process 2 to dismiss wizard and show desktop.
    FirstBootConfigApplied {
        user_name: String,
        locale: String,
        ui_scale_factor: f32,
    },
}

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
    ListDirectory {
        request_id: u64,
        path: String,
    },

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
}
