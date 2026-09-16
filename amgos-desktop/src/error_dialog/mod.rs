//! Plain English Error Handling & Translation Layer
//!
//! Enforces:
//! - Week 8 Day 3: Error handling system — plain English pop-up, Copy and Report buttons, every error surface
//! - Week 8 Day 4: Plain English error translation layer — map all system errors to user-readable strings
//!
//! Rule: The user should never see raw errno numbers, cryptic backtraces, or Linux subsystem jargon.
//! Every error is mapped into clear, conversational plain English with Copy and Report actions.

use serde::{Deserialize, Serialize};

/// High-level categorization of system faults
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemErrorCategory {
    Network,
    Storage,
    Audio,
    Bluetooth,
    Permission,
    PackageInstall,
    Hardware,
    Generic,
}

/// A structured error ready for human presentation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanReadableError {
    pub category: SystemErrorCategory,
    pub title: String,
    pub explanation: String,
    pub technical_detail: String,
    pub suggestion: String,
}

impl HumanReadableError {
    pub fn new(
        category: SystemErrorCategory,
        title: &str,
        explanation: &str,
        technical_detail: &str,
        suggestion: &str,
    ) -> Self {
        Self {
            category,
            title: title.to_string(),
            explanation: explanation.to_string(),
            technical_detail: technical_detail.to_string(),
            suggestion: suggestion.to_string(),
        }
    }
}

/// Plain English Error Translation Layer
pub struct ErrorTranslator;

impl ErrorTranslator {
    /// Translates raw POSIX errno or internal string error into a HumanReadableError
    pub fn translate(raw_error: &str) -> HumanReadableError {
        let trimmed = raw_error.trim();

        if trimmed.contains("EACCES") || trimmed.contains("Permission denied") {
            HumanReadableError {
                category: SystemErrorCategory::Permission,
                title: "Permission Required".to_string(),
                explanation: "AMG-OS prevented access to a protected system area.".to_string(),
                technical_detail: trimmed.to_string(),
                suggestion: "This action requires administrative confirmation in Settings."
                    .to_string(),
            }
        } else if trimmed.contains("ENOENT") || trimmed.contains("No such file") {
            HumanReadableError {
                category: SystemErrorCategory::Storage,
                title: "Item Not Found".to_string(),
                explanation: "The file or folder could not be located on this machine.".to_string(),
                technical_detail: trimmed.to_string(),
                suggestion: "It may have been moved or deleted. Use Pathfinder to search for it."
                    .to_string(),
            }
        } else if trimmed.contains("ENOSPC") || trimmed.contains("No space left on device") {
            HumanReadableError {
                category: SystemErrorCategory::Storage,
                title: "Storage Space Full".to_string(),
                explanation:
                    "There is not enough storage space remaining to complete this operation."
                        .to_string(),
                technical_detail: trimmed.to_string(),
                suggestion: "Remove unused downloads or applications in Filer to free up space."
                    .to_string(),
            }
        } else if trimmed.contains("ECONNREFUSED") || trimmed.contains("Connection refused") {
            HumanReadableError {
                category: SystemErrorCategory::Network,
                title: "Unable to Connect".to_string(),
                explanation: "The network server or device refused the connection.".to_string(),
                technical_detail: trimmed.to_string(),
                suggestion: "Verify that the network is active and try again.".to_string(),
            }
        } else if trimmed.contains("ETIMEDOUT") || trimmed.contains("timed out") {
            HumanReadableError {
                category: SystemErrorCategory::Network,
                title: "Connection Timed Out".to_string(),
                explanation: "The network took too long to respond.".to_string(),
                technical_detail: trimmed.to_string(),
                suggestion: "Check your Wi-Fi signal in the top bar and try again.".to_string(),
            }
        } else if trimmed.contains("SHA256 mismatch") || trimmed.contains("checksum mismatch") {
            HumanReadableError {
                category: SystemErrorCategory::PackageInstall,
                title: "Download Verification Failed".to_string(),
                explanation: "The downloaded file was corrupted or altered during transfer."
                    .to_string(),
                technical_detail: trimmed.to_string(),
                suggestion:
                    "AMG-OS blocked this installation for security. Please download it again."
                        .to_string(),
            }
        } else if trimmed.contains("PipeWire") || trimmed.contains("Audio") {
            HumanReadableError {
                category: SystemErrorCategory::Audio,
                title: "Audio Service Recovering".to_string(),
                explanation:
                    "An issue occurred with the sound subsystem, but AMG-OS is resetting it."
                        .to_string(),
                technical_detail: trimmed.to_string(),
                suggestion: "Sound will resume momentarily. If not, toggle output in Settings."
                    .to_string(),
            }
        } else if trimmed.contains("Bluetooth") || trimmed.contains("bluetoothctl") {
            HumanReadableError {
                category: SystemErrorCategory::Bluetooth,
                title: "Bluetooth Device Disconnected".to_string(),
                explanation: "Communication with the wireless device was interrupted.".to_string(),
                technical_detail: trimmed.to_string(),
                suggestion:
                    "Ensure the device is powered on and within range, then reconnect in Settings."
                        .to_string(),
            }
        } else {
            HumanReadableError {
                category: SystemErrorCategory::Generic,
                title: "Something Went Wrong".to_string(),
                explanation: "An unexpected condition occurred while processing this request.".to_string(),
                technical_detail: trimmed.to_string(),
                suggestion: "You can copy the error details below to submit a diagnostic report via Astrophage.".to_string(),
            }
        }
    }
}

/// State of an active Error Dialog pop-up on the desktop
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorDialogState {
    pub is_open: bool,
    pub error: HumanReadableError,
    pub copy_button_hovered: bool,
    pub report_button_hovered: bool,
    pub dismiss_button_hovered: bool,
    pub copied_feedback_timer: f32,
}

impl ErrorDialogState {
    pub fn new(error: HumanReadableError) -> Self {
        Self {
            is_open: true,
            error,
            copy_button_hovered: false,
            report_button_hovered: false,
            dismiss_button_hovered: false,
            copied_feedback_timer: 0.0,
        }
    }

    pub fn from_raw(raw_error: &str) -> Self {
        Self::new(ErrorTranslator::translate(raw_error))
    }

    pub fn dismiss(&mut self) {
        self.is_open = false;
    }

    pub fn copy_to_clipboard(&mut self) -> String {
        self.copied_feedback_timer = 2.0; // Show feedback for 2 seconds
        format!(
            "AMG-OS Error Report\nTitle: {}\nCategory: {:?}\nExplanation: {}\nSuggestion: {}\nTechnical Detail:\n{}",
            self.error.title, self.error.category, self.error.explanation, self.error.suggestion, self.error.technical_detail
        )
    }

    pub fn tick(&mut self, dt: f32) {
        if self.copied_feedback_timer > 0.0 {
            self.copied_feedback_timer = (self.copied_feedback_timer - dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_translation() {
        let e1 = ErrorTranslator::translate("os error 13: Permission denied (EACCES)");
        assert_eq!(e1.category, SystemErrorCategory::Permission);
        assert_eq!(e1.title, "Permission Required");

        let e2 = ErrorTranslator::translate("os error 2: No such file or directory (ENOENT)");
        assert_eq!(e2.category, SystemErrorCategory::Storage);
        assert_eq!(e2.title, "Item Not Found");

        let e3 = ErrorTranslator::translate("SHA256 mismatch on download stage");
        assert_eq!(e3.category, SystemErrorCategory::PackageInstall);
        assert_eq!(e3.title, "Download Verification Failed");
    }

    #[test]
    fn test_dialog_lifecycle_and_copy() {
        let mut dialog = ErrorDialogState::from_raw("ETIMEDOUT: network request timed out");
        assert!(dialog.is_open);
        assert_eq!(dialog.error.title, "Connection Timed Out");

        let clipboard = dialog.copy_to_clipboard();
        assert!(clipboard.contains("Connection Timed Out"));
        assert!(clipboard.contains("ETIMEDOUT"));
        assert!(dialog.copied_feedback_timer > 0.0);

        dialog.tick(3.0);
        assert_eq!(dialog.copied_feedback_timer, 0.0);

        dialog.dismiss();
        assert!(!dialog.is_open);
    }
}
