//! Pilot Control — Virtual Desktop Manager (Process 2)
//!
//! Pilot Control is AMGOS's Mission Control equivalent.
//! Layout per spec Section 3.3:
//!   Left sidebar: virtual desktop list. Each desktop is a named slot.
//!   A + icon at the bottom creates a new virtual desktop.
//!   Dragging an app thumbnail onto the + icon creates a new desktop and
//!   moves the app into it.
//!   Center: all open app and window thumbnails for the current virtual desktop.
//!           Full, accurate previews. Not placeholder icons.
//!
//! Interaction:
//!   Drag and drop from center to any desktop in the left sidebar.
//!   Drag to + creates a new desktop and completes the move.
//!   Click a desktop in the sidebar to switch to it.
//!   Click a thumbnail in the center to bring that app to focus.
//!
//! Input:
//!   Triggered by MotionWave gesture (multi-finger swipe, configurable).
//!   Navigable by keyboard through MotionWave keyboard routing.
//!
//! Animation:
//!   Entry and exit use cubic-bezier timing (DECELERATE in, ACCELERATE out).
//!   Virtual desktop directional transitions use STANDARD curve.

use crate::animations::curves;

/// Unique identifier for a virtual desktop slot
pub type DesktopId = u32;

/// A window's thumbnail in Pilot Control (actual window content preview)
#[derive(Debug, Clone, PartialEq)]
pub struct WindowThumbnail {
    pub window_id: u64,
    pub app_id: String,
    pub app_title: String,
    /// Logical geometry in the compositor coordinate space
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Whether this window is currently focused
    pub is_focused: bool,
    /// Accurate rendered bitmap preview buffer (RGBA8, 32bpp)
    pub preview_rgba: Vec<u8>,
    pub preview_width: u32,
    pub preview_height: u32,
}

impl WindowThumbnail {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window_id: u64,
        app_id: &str,
        app_title: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        is_focused: bool,
    ) -> Self {
        let pw = 320u32;
        let ph = 200u32;
        let mut preview = vec![18u8; (pw * ph * 4) as usize];
        // Title bar titanium (#1c1c1f)
        for y_coord in 0..24.min(ph) {
            for x_coord in 0..pw {
                let idx = ((y_coord * pw + x_coord) * 4) as usize;
                preview[idx] = 28;
                preview[idx + 1] = 28;
                preview[idx + 2] = 31;
                preview[idx + 3] = 255;
            }
        }
        Self {
            window_id,
            app_id: app_id.to_string(),
            app_title: app_title.to_string(),
            x,
            y,
            width,
            height,
            is_focused,
            preview_rgba: preview,
            preview_width: pw,
            preview_height: ph,
        }
    }

    pub fn update_preview(&mut self, rgba: Vec<u8>, width: u32, height: u32) {
        self.preview_rgba = rgba;
        self.preview_width = width;
        self.preview_height = height;
    }
}

/// A virtual desktop slot — owns a set of windows
#[derive(Debug, Clone)]
pub struct VirtualDesktop {
    pub id: DesktopId,
    pub name: String,
    pub windows: Vec<WindowThumbnail>,
}

impl VirtualDesktop {
    pub fn new(id: DesktopId, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            windows: Vec::new(),
        }
    }
}

/// Direction of a virtual desktop transition (matches sidebar position relationship)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DesktopTransitionDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Pilot Control animation state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PilotControlState {
    Hidden,
    Entering,
    Active,
    Exiting,
}

pub struct PilotControl {
    pub state: PilotControlState,
    pub desktops: Vec<VirtualDesktop>,
    pub active_desktop_id: DesktopId,
    pub next_desktop_id: DesktopId,

    /// Animation progress [0.0, 1.0] for Pilot Control overlay entry/exit
    overlay_animation: f64,
    /// Animation progress for in-progress virtual desktop transitions
    transition_animation: Option<DesktopTransitionAnimation>,
}

struct DesktopTransitionAnimation {
    _from_id: DesktopId,
    _to_id: DesktopId,
    direction: DesktopTransitionDirection,
    progress: f64,
}

impl PilotControl {
    pub fn new() -> Self {
        let initial_desktop = VirtualDesktop::new(1, "Desktop 1");
        Self {
            state: PilotControlState::Hidden,
            desktops: vec![initial_desktop],
            active_desktop_id: 1,
            next_desktop_id: 2,
            overlay_animation: 0.0,
            transition_animation: None,
        }
    }

    /// Toggle Pilot Control visibility (triggered by MotionWave gesture)
    pub fn toggle(&mut self) {
        match self.state {
            PilotControlState::Hidden => {
                self.state = PilotControlState::Entering;
                self.overlay_animation = 0.0;
            }
            PilotControlState::Active => {
                self.state = PilotControlState::Exiting;
            }
            _ => {}
        }
    }

    /// Open Pilot Control
    pub fn show(&mut self) {
        if matches!(self.state, PilotControlState::Hidden) {
            self.state = PilotControlState::Entering;
            self.overlay_animation = 0.0;
        }
    }

    /// Close Pilot Control
    pub fn hide(&mut self) {
        if matches!(
            self.state,
            PilotControlState::Active | PilotControlState::Entering
        ) {
            self.state = PilotControlState::Exiting;
        }
    }

    /// Create a new virtual desktop. Returns the new desktop's ID.
    pub fn create_desktop(&mut self) -> DesktopId {
        let id = self.next_desktop_id;
        self.next_desktop_id += 1;
        let name = format!("Desktop {}", self.desktops.len() + 1);
        self.desktops.push(VirtualDesktop::new(id, &name));
        id
    }

    /// Switch to a specific virtual desktop, recording direction for animation
    pub fn switch_to(&mut self, target_id: DesktopId) {
        if target_id == self.active_desktop_id {
            return;
        }

        let current_pos = self
            .desktops
            .iter()
            .position(|d| d.id == self.active_desktop_id);
        let target_pos = self.desktops.iter().position(|d| d.id == target_id);

        let direction = match (current_pos, target_pos) {
            (Some(c), Some(t)) if t < c => DesktopTransitionDirection::Up,
            (Some(c), Some(t)) if t > c => DesktopTransitionDirection::Down,
            _ => DesktopTransitionDirection::Right,
        };
        self.transition_animation = Some(DesktopTransitionAnimation {
            _from_id: self.active_desktop_id,
            _to_id: target_id,
            direction,
            progress: 0.0,
        });
        self.active_desktop_id = target_id;
    }

    /// Move a window from one desktop to another (drag-and-drop in Pilot Control)
    pub fn move_window_to_desktop(
        &mut self,
        window_id: u64,
        from_desktop_id: DesktopId,
        to_desktop_id: DesktopId,
    ) -> bool {
        // Find and remove from source desktop
        let window = {
            let src = self.desktops.iter_mut().find(|d| d.id == from_desktop_id);
            if let Some(desktop) = src {
                let pos = desktop
                    .windows
                    .iter()
                    .position(|w| w.window_id == window_id);
                pos.map(|p| desktop.windows.remove(p))
            } else {
                None
            }
        };

        // Add to target desktop
        if let Some(w) = window {
            let dst = self.desktops.iter_mut().find(|d| d.id == to_desktop_id);
            if let Some(desktop) = dst {
                desktop.windows.push(w);
                return true;
            }
        }
        false
    }

    /// Move window to a new desktop (drag to + icon interaction)
    pub fn move_window_to_new_desktop(
        &mut self,
        window_id: u64,
        from_desktop_id: DesktopId,
    ) -> DesktopId {
        let new_id = self.create_desktop();
        self.move_window_to_desktop(window_id, from_desktop_id, new_id);
        self.switch_to(new_id);
        new_id
    }

    /// Register a window on the active desktop (called by compositor surface manager)
    pub fn on_window_opened(&mut self, window: WindowThumbnail) {
        if let Some(desktop) = self
            .desktops
            .iter_mut()
            .find(|d| d.id == self.active_desktop_id)
        {
            desktop.windows.push(window);
        }
    }

    /// Remove a window from all desktops (called by compositor surface manager on close)
    pub fn on_window_closed(&mut self, window_id: u64) {
        for desktop in self.desktops.iter_mut() {
            desktop.windows.retain(|w| w.window_id != window_id);
        }
    }

    /// Get the currently active desktop
    pub fn active_desktop(&self) -> Option<&VirtualDesktop> {
        self.desktops
            .iter()
            .find(|d| d.id == self.active_desktop_id)
    }

    /// Animate the overlay and desktop transitions. Call every compositor frame.
    /// Returns (overlay_opacity, transition_offset_x, transition_offset_y)
    pub fn tick(&mut self, delta_t: f64) -> (f64, f64, f64) {
        let anim_duration = 0.30; // 300ms for overlay entry/exit

        let overlay_opacity = match self.state {
            PilotControlState::Entering => {
                self.overlay_animation =
                    (self.overlay_animation + delta_t / anim_duration).min(1.0);
                if self.overlay_animation >= 1.0 {
                    self.state = PilotControlState::Active;
                }
                curves::DECELERATE.solve(self.overlay_animation)
            }
            PilotControlState::Exiting => {
                self.overlay_animation =
                    (self.overlay_animation - delta_t / anim_duration).max(0.0);
                if self.overlay_animation <= 0.0 {
                    self.state = PilotControlState::Hidden;
                }
                curves::ACCELERATE.solve(self.overlay_animation)
            }
            PilotControlState::Active => 1.0,
            PilotControlState::Hidden => 0.0,
        };

        // Animate desktop transitions
        let (tx, ty) = if let Some(ref mut anim) = self.transition_animation {
            anim.progress = (anim.progress + delta_t / 0.25).min(1.0);
            let eased = curves::STANDARD.solve(anim.progress);
            let offset = 1.0 - eased;
            let (ox, oy) = match anim.direction {
                DesktopTransitionDirection::Up => (0.0, -offset),
                DesktopTransitionDirection::Down => (0.0, offset),
                DesktopTransitionDirection::Left => (-offset, 0.0),
                DesktopTransitionDirection::Right => (offset, 0.0),
            };
            if anim.progress >= 1.0 {
                self.transition_animation = None;
            }
            (ox, oy)
        } else {
            (0.0, 0.0)
        };

        (overlay_opacity, tx, ty)
    }

    pub fn is_visible(&self) -> bool {
        !matches!(self.state, PilotControlState::Hidden)
    }

    /// Renders Pilot Control overlay into the compositor framebuffer
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn render_overlay(&self, display_width: u32, display_height: u32, framebuffer: &mut [u8]) {
        if self.state == PilotControlState::Hidden {
            return;
        }

        let opacity = match self.state {
            PilotControlState::Entering | PilotControlState::Exiting => self.overlay_animation,
            PilotControlState::Active => 1.0,
            PilotControlState::Hidden => 0.0,
        };

        if opacity <= 0.001 {
            return;
        }

        let dw = display_width as usize;
        let dh = display_height as usize;

        // 1. Dark frosted glass scrim overlay across entire screen
        let dim_factor = (opacity * 0.75) as f32;
        for chunk in framebuffer.chunks_exact_mut(4) {
            chunk[0] = (chunk[0] as f32 * (1.0 - dim_factor * 0.7)) as u8;
            chunk[1] = (chunk[1] as f32 * (1.0 - dim_factor * 0.7)) as u8;
            chunk[2] = (chunk[2] as f32 * (1.0 - dim_factor * 0.7)) as u8;
        }

        // 2. Left sidebar: Virtual Desktop list (240px wide)
        let sidebar_w = (240.0 * opacity) as usize;
        for y in 0..dh {
            for x in 0..sidebar_w.min(dw) {
                let idx = (y * dw + x) * 4;
                if idx + 3 < framebuffer.len() {
                    // Titanium dark glass background (#141416)
                    framebuffer[idx] = 20;
                    framebuffer[idx + 1] = 20;
                    framebuffer[idx + 2] = 22;
                    framebuffer[idx + 3] = 255;
                }
            }
        }

        // 3. Render Virtual Desktop slots in sidebar
        let slot_height = 80;
        let slot_padding = 16;
        for (i, desktop) in self.desktops.iter().enumerate() {
            let slot_y = 60 + i * (slot_height + slot_padding);
            if slot_y + slot_height >= dh {
                break;
            }

            let is_active = desktop.id == self.active_desktop_id;
            let border_color = if is_active {
                [255u8, 85, 0, 255] // Space Orange
            } else {
                [45u8, 45, 50, 255]
            };

            for sy in slot_y..(slot_y + slot_height) {
                for sx in 16..sidebar_w.saturating_sub(16).min(dw) {
                    let idx = (sy * dw + sx) * 4;
                    if idx + 3 < framebuffer.len() {
                        let is_border = sy == slot_y
                            || sy == slot_y + slot_height - 1
                            || sx == 16
                            || sx == sidebar_w.saturating_sub(17);
                        if is_border {
                            framebuffer[idx..idx + 4].copy_from_slice(&border_color);
                        } else {
                            framebuffer[idx] = 28;
                            framebuffer[idx + 1] = 28;
                            framebuffer[idx + 2] = 32;
                            framebuffer[idx + 3] = 255;
                        }
                    }
                }
            }
        }

        // 4. Center Area: Render window thumbnail previews for active desktop
        if let Some(active) = self.active_desktop() {
            let center_x_start = sidebar_w + 40;
            let center_w = dw.saturating_sub(center_x_start + 40);
            if center_w == 0 || active.windows.is_empty() {
                return;
            }

            let thumb_w = 320usize;
            let thumb_h = 200usize;
            let cols = (center_w / (thumb_w + 24)).max(1);

            for (w_idx, win) in active.windows.iter().enumerate() {
                let col = w_idx % cols;
                let row = w_idx / cols;
                let tx = center_x_start + col * (thumb_w + 24);
                let ty = 80 + row * (thumb_h + 40);

                if ty + thumb_h >= dh || tx + thumb_w >= dw {
                    continue;
                }

                // Blit thumbnail preview pixels directly
                let pw = win.preview_width as usize;
                let ph = win.preview_height as usize;
                if !win.preview_rgba.is_empty() && pw > 0 && ph > 0 {
                    for py in 0..thumb_h.min(ph) {
                        for px in 0..thumb_w.min(pw) {
                            let src_idx = (py * pw + px) * 4;
                            let dst_idx = ((ty + py) * dw + (tx + px)) * 4;
                            if src_idx + 3 < win.preview_rgba.len()
                                && dst_idx + 3 < framebuffer.len()
                            {
                                framebuffer[dst_idx..dst_idx + 4]
                                    .copy_from_slice(&win.preview_rgba[src_idx..src_idx + 4]);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Default for PilotControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let pc = PilotControl::new();
        assert_eq!(pc.desktops.len(), 1);
        assert_eq!(pc.active_desktop_id, 1);
        assert_eq!(pc.state, PilotControlState::Hidden);
    }

    #[test]
    fn test_create_desktop() {
        let mut pc = PilotControl::new();
        let id = pc.create_desktop();
        assert_eq!(id, 2);
        assert_eq!(pc.desktops.len(), 2);
    }

    #[test]
    fn test_switch_desktop() {
        let mut pc = PilotControl::new();
        pc.create_desktop();
        pc.switch_to(2);
        assert_eq!(pc.active_desktop_id, 2);
        assert!(pc.transition_animation.is_some());
    }

    #[test]
    fn test_window_move_between_desktops() {
        let mut pc = PilotControl::new();
        pc.create_desktop();

        let window = WindowThumbnail::new(1001, "terminow", "Terminow", 100, 100, 800, 600, true);
        pc.desktops[0].windows.push(window);

        let moved = pc.move_window_to_desktop(1001, 1, 2);
        assert!(moved);
        assert_eq!(pc.desktops[0].windows.len(), 0);
        assert_eq!(pc.desktops[1].windows.len(), 1);
    }

    #[test]
    fn test_move_to_new_desktop() {
        let mut pc = PilotControl::new();
        let window = WindowThumbnail::new(2001, "filer", "Filer", 0, 0, 1200, 800, false);
        pc.desktops[0].windows.push(window);
        let new_id = pc.move_window_to_new_desktop(2001, 1);
        assert_eq!(new_id, 2);
        assert_eq!(pc.active_desktop_id, 2);
    }

    #[test]
    fn test_pilot_control_toggle() {
        let mut pc = PilotControl::new();
        pc.toggle();
        assert_eq!(pc.state, PilotControlState::Entering);
    }

    #[test]
    fn test_window_close_cleanup() {
        let mut pc = PilotControl::new();
        pc.create_desktop();
        let window =
            WindowThumbnail::new(3001, "zen-browser", "Zen Browser", 0, 0, 1920, 1080, true);
        pc.on_window_opened(window);
        pc.on_window_closed(3001);
        assert!(pc.desktops.iter().all(|d| d.windows.is_empty()));
    }

    #[test]
    fn test_render_overlay() {
        let mut pc = PilotControl::new();
        pc.show();
        pc.tick(0.30); // Advance to Active state
        assert_eq!(pc.state, PilotControlState::Active);

        let window = WindowThumbnail::new(1001, "terminow", "Terminow", 100, 100, 800, 600, true);
        pc.on_window_opened(window);

        let mut fb = vec![0u8; 1920 * 1080 * 4];
        pc.render_overlay(1920, 1080, &mut fb);
        // Scrim and sidebar pixels should be modified
        assert!(fb[0] != 0 || fb[1] != 0 || fb[2] != 0 || fb[3] != 0);
    }
}
