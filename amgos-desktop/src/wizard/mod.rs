//! Setup Wizard Subsystem
//!
//! Owns the first-boot onboarding experience.
//! Strictly sequential: one screen, one question.
//! Dispatches the completed signed configuration to Process 1 e-bus.
//!
//! Visual Standard: Full-screen Wayland surface. Centered card layout.
//! macOS-style "Continue" buttons. Young Serif typeface for body copy.
//! Cubic-bezier animation transitions between steps.


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

/// Represents a UI node in the Setup Wizard card layout.
#[derive(Debug, Clone)]
pub struct WizardUINode {
    pub text: String,
    pub typeface: String, // "Young Serif", "Inter"
    pub font_size: f32,
    pub opacity: f64,
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

    // Animation state
    pub card_opacity: f64,
    pub card_y_offset: f64,
    pub transition_progress: f64,
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

            card_opacity: 0.0,
            card_y_offset: 20.0,
            transition_progress: 0.0,
        }
    }

    /// Ticks the animation engine using cubic-bezier curve timing.
    pub fn tick(&mut self, delta_t: f64) {
        if self.transition_progress < 1.0 {
            // Cubic bezier ease-out approximation for the transition
            self.transition_progress = (self.transition_progress + delta_t * 2.0).min(1.0);
            
            // Standard Animation Engine curve
            let ease_out = 1.0 - (1.0 - self.transition_progress).powi(3);
            
            self.card_opacity = ease_out;
            self.card_y_offset = 20.0 * (1.0 - ease_out);
        }
    }

    /// Advance to the next wizard step. Enforces validation rules.
    pub fn advance(&mut self) -> Result<WizardStep, &'static str> {
        match self.current_step {
            WizardStep::LanguageAndRegion => {
                self.current_step = WizardStep::Network;
                self.start_transition();
                Ok(self.current_step)
            }
            WizardStep::Network => {
                if !self.network_connected {
                    return Err("Network must be connected before proceeding");
                }
                self.current_step = WizardStep::UserAccount;
                self.start_transition();
                Ok(self.current_step)
            }
            WizardStep::UserAccount => {
                if self.user_name.trim().is_empty() {
                    return Err("User account name is required");
                }
                self.current_step = WizardStep::PrivacyAndTelemetry;
                self.start_transition();
                Ok(self.current_step)
            }
            WizardStep::PrivacyAndTelemetry => {
                self.current_step = WizardStep::DisplayAndAccessibility;
                self.start_transition();
                Ok(self.current_step)
            }
            WizardStep::DisplayAndAccessibility => {
                self.current_step = WizardStep::Migration;
                self.start_transition();
                Ok(self.current_step)
            }
            WizardStep::Migration => {
                self.current_step = WizardStep::Done;
                self.is_completed = true;
                self.start_transition();
                Ok(self.current_step)
            }
            WizardStep::Done => Ok(WizardStep::Done),
        }
    }

    fn start_transition(&mut self) {
        self.transition_progress = 0.0;
        self.card_opacity = 0.0;
        self.card_y_offset = -20.0;
    }

    /// Generates the UI nodes for the current step, respecting typography policies.
    pub fn render_nodes(&self) -> Vec<WizardUINode> {
        let mut nodes = Vec::new();
        let title_text = match self.current_step {
            WizardStep::LanguageAndRegion => "Language & Region",
            WizardStep::Network => "Select a Wi-Fi Network",
            WizardStep::UserAccount => "Create Your Account",
            WizardStep::PrivacyAndTelemetry => "Privacy & Diagnostics",
            WizardStep::DisplayAndAccessibility => "Make It Yours",
            WizardStep::Migration => "Migration Assistant",
            WizardStep::Done => "You're All Set",
        };

        // Title uses Inter (Display)
        nodes.push(WizardUINode {
            text: title_text.to_string(),
            typeface: "Inter".to_string(),
            font_size: 32.0,
            opacity: self.card_opacity,
        });

        // Body uses Young Serif (Primary/Body)
        nodes.push(WizardUINode {
            text: "AMG-OS will use this to configure your system. Set it up once, and it works after that.".to_string(),
            typeface: "Young Serif".to_string(),
            font_size: 16.0,
            opacity: self.card_opacity,
        });

        nodes
    }
}

impl Default for SetupWizardState {
    fn default() -> Self {
        Self::new()
    }
}
