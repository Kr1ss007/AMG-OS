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

    /// Response to a Pathfinder search query
    PathfinderResults {
        query_id: u64,
        results: Vec<PathfinderItem>,
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

    /// Submit network credentials securely to Process 1 vault
    ConnectWifi { ssid: String, passphrase: String },

    /// Request diagnostic history from Astrophage
    GetAstrophageBuffer { max_entries: usize },

    /// Apply signed system configuration produced by Settings
    ApplySignedConfig {
        config_payload: Vec<u8>,
        hmac_signature: [u8; 32],
    },
}
