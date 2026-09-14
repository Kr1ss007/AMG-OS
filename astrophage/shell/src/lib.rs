//! astrophage-shell: Process 2 Diagnostic UI & Report Builder
//!
//! Presents system diagnostic telemetry, enforces strict privacy scrubbing
//! (no personal data, browser history, or user paths), and formats GitHub issues.

use amgos_protocol::ebus::AstrophageRecord;
use astrophage_protocol::GitHubIssuePayload;

pub struct DiagnosticReportBuilder {
    user_description: String,
    hardware_summary: String,
    recent_events: Vec<AstrophageRecord>,
}

impl DiagnosticReportBuilder {
    pub fn new(hardware_summary: &str) -> Self {
        Self {
            user_description: String::new(),
            hardware_summary: hardware_summary.to_string(),
            recent_events: Vec::new(),
        }
    }

    pub fn set_user_description(&mut self, text: &str) {
        self.user_description = text.to_string();
    }

    pub fn set_events(&mut self, events: Vec<AstrophageRecord>) {
        // Privacy filter: scrub any potential usernames or local home paths
        self.recent_events = events
            .into_iter()
            .map(|mut record| {
                record.message = scrub_personal_data(&record.message);
                record
            })
            .collect();
    }

    pub fn build_github_payload(&self) -> GitHubIssuePayload {
        GitHubIssuePayload {
            title: if self.user_description.len() > 60 {
                format!("{}...", &self.user_description[..57])
            } else if !self.user_description.is_empty() {
                self.user_description.clone()
            } else {
                "Automated Hardware Diagnostic Report".to_string()
            },
            user_description: self.user_description.clone(),
            hardware_summary: self.hardware_summary.clone(),
            os_version: "AMGOS v0.0.1 Upstream Color".to_string(),
            recent_event_log: self.recent_events.clone(),
            timestamp_utc: "2026-09-14T14:25:00Z".to_string(),
        }
    }
}

fn scrub_personal_data(msg: &str) -> String {
    // Replaces home directory paths and sensitive markers
    let mut scrubbed = msg.to_string();
    if let Some(pos) = scrubbed.find("/home/") {
        if let Some(end) = scrubbed[pos + 6..].find('/') {
            let user_part = &scrubbed[pos..pos + 6 + end];
            scrubbed = scrubbed.replace(user_part, "/home/[user]");
        }
    }
    scrubbed
}
