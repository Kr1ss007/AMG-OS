//! Filer Protocol: Core <-> Shell Shared Contracts
//!
//! Contract between Filer Core (Process 1 backend) and Filer Shell (Process 2 UI).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: u64,
    pub modified_timestamp_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilerRequest {
    ListDirectory { path: String },
    PathfinderSearch { query_id: u64, query: String },
    InspectPackage { package_path: String },
    RequestInstall { package_path: String },
    RequestUninstall { app_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilerResponse {
    DirectoryListing {
        path: String,
        entries: Vec<FileEntry>,
    },
    PathfinderResults {
        query_id: u64,
        items: Vec<amgos_protocol::ebus::PathfinderItem>,
    },
    InspectionResult {
        report: amgos_protocol::ebus::InspectionReport,
    },
    ActionFinished {
        success: bool,
        message: String,
    },
}
