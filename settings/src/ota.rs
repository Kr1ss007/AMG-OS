//! Over-The-Air (OTA) Update Controller
//!
//! Exposes update management through Settings.
//! Never exposes ISOs, URLs, or command lines to the user.
//! Coordinates with A/B partition swap mechanism.

pub const OS_VERSION: &str = "0.0.1";
pub const OS_CODENAME: &str = "Upstream Color";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateType {
    SecurityPatch,
    FeatureUpdate,
    MajorVersion,
}

#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    pub version: String,
    pub update_type: UpdateType,
    pub title: String,
    pub description: String,
    pub size_mb: u32,
}

#[derive(Debug, Clone)]
pub struct OtaUpdateState {
    pub current_version: &'static str,
    pub current_codename: &'static str,
    pub update_available: Option<AvailableUpdate>,
    pub download_progress_percent: f32,
    pub is_downloading: bool,
    pub is_ready_to_install: bool,
}

impl OtaUpdateState {
    pub fn new() -> Self {
        Self {
            current_version: OS_VERSION,
            current_codename: OS_CODENAME,
            update_available: None,
            download_progress_percent: 0.0,
            is_downloading: false,
            is_ready_to_install: false,
        }
    }

    pub fn check_for_updates(&mut self) -> Option<&AvailableUpdate> {
        // In local / alpha testing, returns none or staged update
        self.update_available.as_ref()
    }
}

impl Default for OtaUpdateState {
    fn default() -> Self {
        Self::new()
    }
}
