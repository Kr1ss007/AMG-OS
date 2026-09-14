//! astrophage-protocol: Shared Types for System Diagnostics
//!
//! Owns telemetry record structures and pre-formatted GitHub issue payloads.

use amgos_protocol::ebus::AstrophageRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitHubIssuePayload {
    pub title: String,
    pub user_description: String,
    pub hardware_summary: String,
    pub os_version: String,
    pub recent_event_log: Vec<AstrophageRecord>,
    pub timestamp_utc: String,
}

impl GitHubIssuePayload {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Diagnostic Report: {}\n\n", self.title));
        md.push_str("### User Description\n");
        md.push_str(&format!("{}\n\n", self.user_description));
        md.push_str("### System Environment\n");
        md.push_str(&format!("- **OS Version:** {}\n", self.os_version));
        md.push_str(&format!("- **Hardware:** {}\n", self.hardware_summary));
        md.push_str(&format!("- **Timestamp:** {}\n\n", self.timestamp_utc));
        md.push_str("### Relevant Event Buffer (Latest)\n```text\n");
        for record in &self.recent_event_log {
            md.push_str(&format!(
                "[{:?}] [{}] {}\n",
                record.level, record.subsystem, record.message
            ));
        }
        md.push_str("```\n");
        md
    }
}
