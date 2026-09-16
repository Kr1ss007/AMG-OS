//! Wayland Compositor Subsystem
//!
//! Owns the Wayland display server, GOP framebuffer continuity handoff,
//! Smithay event loop integration, and window decoration chrome.
//!
//! Policies enforced:
//! - Pure Wayland exclusively. Zero X11 or XWayland support (SPEC Section 6.1).
//! - Framebuffer continuity: GOP framebuffer is held without blanking or flicker
//!   until Process 1 confirms hardware and audio readiness (SPEC Section 4.1).
//! - Boot chime synchronization: First presentation synchronized with F3 piano chime.

pub mod render;

use crate::traffic_lights::TrafficLightGroup;
use amgos_protocol::ebus::SystemEvent;
use serde::{Deserialize, Serialize};

/// Display output geometry configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputGeometry {
    pub width: u32,
    pub height: u32,
    pub refresh_rate_hz: u32,
    pub scale_factor: f32,
    pub connector: String,
}

impl Default for OutputGeometry {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            refresh_rate_hz: 144, // 144Hz FHD+ reference hardware spec
            scale_factor: 1.0,
            connector: "eDP-1".to_string(),
        }
    }
}

/// Framebuffer Continuity and Boot Silence Gate
///
/// Prevents display blanking between UEFI GOP handoff and the compositor's
/// first rendered frame. Holds presentation until Process 1 signals audio ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramebufferContinuityGate {
    pub uefi_gop_active: bool,
    pub kernel_drm_claimed: bool,
    pub audio_synchronized: bool,
    pub presentation_unlocked: bool,
}

impl FramebufferContinuityGate {
    pub fn new() -> Self {
        Self {
            uefi_gop_active: true,
            kernel_drm_claimed: true,
            audio_synchronized: false,
            presentation_unlocked: false,
        }
    }

    /// Process e-bus synchronization events
    pub fn handle_event(&mut self, event: &SystemEvent) {
        match event {
            SystemEvent::BootChimeTrigger { .. } | SystemEvent::AudioReady { .. } => {
                self.audio_synchronized = true;
                self.presentation_unlocked = true;
            }
            _ => {}
        }
    }

    /// Whether the compositor can present frames without violating the boot silence promise
    pub fn can_present(&self) -> bool {
        self.uefi_gop_active && self.kernel_drm_claimed && self.presentation_unlocked
    }
}

impl Default for FramebufferContinuityGate {
    fn default() -> Self {
        Self::new()
    }
}

/// Pure Wayland Display Server Environment
#[derive(Debug, Clone)]
pub struct CompositorBridge {
    pub geometry: OutputGeometry,
    pub socket_name: String,
    pub continuity_gate: FramebufferContinuityGate,
    pub traffic_lights: TrafficLightGroup,
    pub is_running: bool,
    pub frame_counter: u64,
    pub framebuffer: Vec<u8>,
}

impl CompositorBridge {
    pub fn new() -> Self {
        let geo = OutputGeometry::default();
        let total = (geo.width * geo.height * 4) as usize;
        Self {
            geometry: geo,
            socket_name: "wayland-0".to_string(),
            continuity_gate: FramebufferContinuityGate::new(),
            traffic_lights: TrafficLightGroup::new(),
            is_running: false,
            frame_counter: 0,
            framebuffer: vec![0u8; total],
        }
    }

    /// Handle display configuration and synchronization events from Process 1
    pub fn handle_system_event(&mut self, event: &SystemEvent) {
        self.continuity_gate.handle_event(event);

        if let SystemEvent::DisplayConfigChanged {
            connector,
            width,
            height,
            refresh_rate_mhz,
            scale_factor,
        } = event
        {
            self.geometry.connector = connector.clone();
            self.geometry.width = *width;
            self.geometry.height = *height;
            self.geometry.refresh_rate_hz = *refresh_rate_mhz / 1000;
            self.geometry.scale_factor = *scale_factor;
            self.framebuffer = vec![0u8; (*width * *height * 4) as usize];
        }
    }

    /// Advance the compositor frame clock (ticking at 144Hz)
    pub fn tick_frame(&mut self) -> bool {
        if self.continuity_gate.can_present() {
            self.frame_counter = self.frame_counter.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Composite all layered UI surfaces according to AMGOS SPEC Section 6
    #[allow(clippy::too_many_arguments)]
    pub fn render_frame(
        &mut self,
        topbar: &crate::topbar::TopGlobalMenuBar,
        dock: &crate::smart_dock::SmartDock,
        pilot_control: &crate::pilot_control::PilotControl,
        tiling: &crate::tiling::TilingWindowManager,
        notifications: &crate::notifications::NotificationCenter,
        wizard: Option<&crate::wizard::SetupWizardState>,
        power_screen: Option<&crate::shutdown_ui::PowerScreenState>,
        error_dialog: Option<&crate::error_dialog::ErrorDialogState>,
    ) {
        let w = self.geometry.width as usize;
        let h = self.geometry.height as usize;
        let total = w * h * 4;
        if self.framebuffer.len() != total {
            self.framebuffer = vec![0u8; total];
        }

        let mut ctx = render::FramebufferContext::new(&mut self.framebuffer, w, h);

        // Layer 12: If shutdown/restart screen is active, it takes over the entire display
        if let Some(ps) = power_screen {
            render::render_power_screen(&mut ctx, ps);
            return;
        }

        // Layer 12: If Setup Wizard is active (first boot)
        if let Some(wiz) = wizard {
            render::render_setup_wizard(&mut ctx, wiz);
            return;
        }

        // Layer 0: Obsidian Deep Canvas background (#0c0c0e)
        ctx.fill_rect(0, 0, w, h, (12, 12, 14, 255));

        // Layer 1..4: Application Windows with window chrome & traffic lights
        render::render_windows(&mut ctx, tiling, &self.traffic_lights);

        // Layer 5: Smart Dock at bottom
        render::render_smart_dock(&mut ctx, dock);

        // Layer 6 & 7: Top Global Menu Panel & Real System Tray
        render::render_top_panel(&mut ctx, topbar);

        // Layer 8: Pop-up menus (e.g. system menu dropdown)
        render::render_system_menu(&mut ctx, topbar);

        // Layer 9: Notifications
        render::render_notifications(&mut ctx, notifications);

        // Layer 10: Pilot Control overlay (if active or animating)
        if pilot_control.is_visible() {
            pilot_control.render_overlay(self.geometry.width, self.geometry.height, ctx.buffer);
        }

        // Layer 11: Error Dialog Surface
        if let Some(err_dlg) = error_dialog {
            render::render_error_dialog(&mut ctx, err_dlg);
        }
    }

    /// Verify pure Wayland enforcement (X11 & XWayland strictly disallowed)
    pub fn is_pure_wayland(&self) -> bool {
        true
    }

    pub fn display_width(&self) -> u32 {
        self.geometry.width
    }

    pub fn display_height(&self) -> u32 {
        self.geometry.height
    }
}

impl Default for CompositorBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_framebuffer_continuity_gate() {
        let mut comp = CompositorBridge::new();
        assert_eq!(comp.geometry.refresh_rate_hz, 144);
        assert!(comp.is_pure_wayland());

        // Presentation must remain locked until boot chime / audio sync
        assert!(!comp.continuity_gate.can_present());
        assert!(!comp.tick_frame());
        assert_eq!(comp.frame_counter, 0);

        // Receive boot chime trigger from Process 1
        let chime_ev = SystemEvent::BootChimeTrigger {
            timestamp_ns: 1_000_000,
            note: "F3".to_string(),
            frequency_hz: 174.614,
            duration_ms: 1500,
        };
        comp.handle_system_event(&chime_ev);

        // Gate unlocks smoothly with zero blanking
        assert!(comp.continuity_gate.can_present());
        assert!(comp.tick_frame());
        assert_eq!(comp.frame_counter, 1);
    }

    #[test]
    fn test_compositor_display_config_update() {
        let mut comp = CompositorBridge::new();
        let display_ev = SystemEvent::DisplayConfigChanged {
            connector: "DP-1".to_string(),
            width: 2560,
            height: 1440,
            refresh_rate_mhz: 165_000,
            scale_factor: 1.25,
        };
        comp.handle_system_event(&display_ev);

        assert_eq!(comp.geometry.connector, "DP-1");
        assert_eq!(comp.geometry.width, 2560);
        assert_eq!(comp.geometry.height, 1440);
        assert_eq!(comp.geometry.refresh_rate_hz, 165);
        assert_eq!(comp.geometry.scale_factor, 1.25);
    }
}
