//! App Installer & Uninstaller UI State
//!
//! Enforces symmetric installation and uninstallation:
//! - Same surface, same gesture, opposite action.
//! - Right-click on any installed app presents Uninstall.
//! - Confirmation dialog adheres strictly to standard:
//!   - Yes (Confirm): Space Orange (#FF5500), Right button.
//!   - No (Cancel): Space White (#F0F0F2), Left button.

use amgos_protocol::ebus::InspectionReport;

pub const CONFIRM_COLOR_YES: &str = "#FF5500"; // Space Orange (Right)
pub const CONFIRM_COLOR_NO: &str = "#F0F0F2"; // Space White (Left)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageActionType {
    Install,
    Uninstall,
}

#[derive(Debug, Clone)]
pub struct PackageActionDialog {
    pub action_type: PackageActionType,
    pub target_name: String,
    pub is_visible: bool,
    pub inspection_report: Option<InspectionReport>,
}

impl PackageActionDialog {
    pub fn new_install(target: &str, report: InspectionReport) -> Self {
        Self {
            action_type: PackageActionType::Install,
            target_name: target.to_string(),
            is_visible: true,
            inspection_report: Some(report),
        }
    }

    pub fn new_uninstall(app_name: &str) -> Self {
        Self {
            action_type: PackageActionType::Uninstall,
            target_name: app_name.to_string(),
            is_visible: true,
            inspection_report: None,
        }
    }

    pub fn button_layout(&self) -> DialogButtonPair {
        DialogButtonPair {
            cancel_label: "Cancel".to_string(),
            cancel_color_hex: CONFIRM_COLOR_NO,
            cancel_is_left: true,
            confirm_label: match self.action_type {
                PackageActionType::Install => "Install".to_string(),
                PackageActionType::Uninstall => "Uninstall".to_string(),
            },
            confirm_color_hex: CONFIRM_COLOR_YES,
            confirm_is_right: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DialogButtonPair {
    pub cancel_label: String,
    pub cancel_color_hex: &'static str,
    pub cancel_is_left: bool,
    pub confirm_label: String,
    pub confirm_color_hex: &'static str,
    pub confirm_is_right: bool,
}
