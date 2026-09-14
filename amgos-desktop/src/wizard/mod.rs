//! Setup Wizard Subsystem
//!
//! Owns the first-boot onboarding experience.
//! Strictly sequential: one screen, one question.
//! Dispatches the completed signed configuration to Process 1 e-bus.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    LanguageAndRegion = 1,
    Network = 2,
    UserAccount = 3,
    PrivacyAndTelemetry = 4,
    DisplayAndAccessibility = 5,
    Migration = 6,
    Done = 7,
}

#[derive(Debug, Clone)]
pub struct SetupWizardState {
    pub current_step: WizardStep,
    pub locale: String,
    pub selected_ssid: Option<String>,
    pub network_connected: bool,
    pub user_name: String,
    pub telemetry_opt_in: bool,
    pub ui_scale_factor: f32,
    pub is_completed: bool,
}

impl SetupWizardState {
    pub fn new() -> Self {
        Self {
            current_step: WizardStep::LanguageAndRegion,
            locale: "en_US.UTF-8".to_string(),
            selected_ssid: None,
            network_connected: false,
            user_name: String::new(),
            telemetry_opt_in: false,
            ui_scale_factor: 1.0,
            is_completed: false,
        }
    }

    /// Advance to the next wizard step. Enforces validation rules.
    pub fn advance(&mut self) -> Result<WizardStep, &'static str> {
        match self.current_step {
            WizardStep::LanguageAndRegion => {
                self.current_step = WizardStep::Network;
                Ok(self.current_step)
            }
            WizardStep::Network => {
                // Network must be confirmed connected before advancing
                if !self.network_connected {
                    return Err("Network must be connected before proceeding");
                }
                self.current_step = WizardStep::UserAccount;
                Ok(self.current_step)
            }
            WizardStep::UserAccount => {
                if self.user_name.trim().is_empty() {
                    return Err("User account name is required");
                }
                self.current_step = WizardStep::PrivacyAndTelemetry;
                Ok(self.current_step)
            }
            WizardStep::PrivacyAndTelemetry => {
                self.current_step = WizardStep::DisplayAndAccessibility;
                Ok(self.current_step)
            }
            WizardStep::DisplayAndAccessibility => {
                self.current_step = WizardStep::Migration;
                Ok(self.current_step)
            }
            WizardStep::Migration => {
                self.current_step = WizardStep::Done;
                self.is_completed = true;
                Ok(self.current_step)
            }
            WizardStep::Done => Ok(WizardStep::Done),
        }
    }
}

impl Default for SetupWizardState {
    fn default() -> Self {
        Self::new()
    }
}
