//! Tiling Window Manager (Process 2 — Animation and Interface Engine)
//!
//! Per spec Section 3.5: Native apps and Wayland app windows support keyboard-driven tiling.
//!
//!   Command + Left   → Tile active window to left half
//!   Command + Right  → Tile active window to right half
//!   Command + Up     → Maximize active window
//!   Command + Down   → Restore / unmaximize active window
//!
//! Tiling is handled by the compositor (Process 2).
//! Shortcuts are registered and routed by MotionWave.
//! Tile animations use the cubic-bezier timing system (STANDARD curve).
//! Applies to native AMGOS apps and Wayland-native apps only.
//! X11 is not supported.

use crate::animations::curves;

/// The logical tile state of a window
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileState {
    /// Window is in its own floating position and size
    Floating,
    /// Window occupies the left half of the display
    LeftHalf,
    /// Window occupies the right half of the display
    RightHalf,
    /// Window is maximized (full display minus panels)
    Maximized,
}

/// A 2D rectangle in compositor coordinate space
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl WindowRect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

/// Tiling animation for a single window's geometry transition
pub struct TileAnimation {
    pub window_id: u64,
    pub from_rect: WindowRect,
    pub to_rect: WindowRect,
    pub progress: f64,
    pub duration: f64,
    pub target_state: TileState,
}

impl TileAnimation {
    pub fn new(
        window_id: u64,
        from: WindowRect,
        to: WindowRect,
        target: TileState,
    ) -> Self {
        Self {
            window_id,
            from_rect: from,
            to_rect: to,
            progress: 0.0,
            duration: 0.20, // 200ms tile animation
            target_state: target,
        }
    }

    /// Advance animation by delta_t. Returns current interpolated rect.
    pub fn tick(&mut self, delta_t: f64) -> WindowRect {
        self.progress = (self.progress + delta_t / self.duration).min(1.0);
        let t = curves::STANDARD.solve(self.progress);

        WindowRect {
            x: lerp_i32(self.from_rect.x, self.to_rect.x, t),
            y: lerp_i32(self.from_rect.y, self.to_rect.y, t),
            width: lerp_u32(self.from_rect.width, self.to_rect.width, t),
            height: lerp_u32(self.from_rect.height, self.to_rect.height, t),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}

fn lerp_i32(a: i32, b: i32, t: f64) -> i32 {
    (a as f64 + (b - a) as f64 * t).round() as i32
}

fn lerp_u32(a: u32, b: u32, t: f64) -> u32 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u32
}

/// Per-window tiling state record
pub struct WindowTilingRecord {
    pub window_id: u64,
    pub tile_state: TileState,
    pub floating_rect: WindowRect, // saved floating geometry for restore
    pub current_rect: WindowRect,
}

/// The tiling window manager — owned by the compositor's surface manager
pub struct TilingWindowManager {
    pub windows: Vec<WindowTilingRecord>,
    pub animations: Vec<TileAnimation>,
    /// Display geometry (minus top panel and dock height)
    pub display_width: u32,
    pub display_height: u32,
    pub panel_height: u32,
    pub dock_height: u32,
}

impl TilingWindowManager {
    pub fn new(display_width: u32, display_height: u32) -> Self {
        Self {
            windows: Vec::new(),
            animations: Vec::new(),
            display_width,
            display_height,
            panel_height: 28, // Top Global Menu Panel height (px)
            dock_height: 64,  // Smart Dock height
        }
    }

    /// Register a new window. All new windows start as Floating.
    pub fn register_window(&mut self, window_id: u64, rect: WindowRect) {
        self.windows.push(WindowTilingRecord {
            window_id,
            tile_state: TileState::Floating,
            floating_rect: rect,
            current_rect: rect,
        });
    }

    /// Remove a window from tracking
    pub fn unregister_window(&mut self, window_id: u64) {
        self.windows.retain(|w| w.window_id != window_id);
        self.animations.retain(|a| a.window_id != window_id);
    }

    /// Apply a tile command to the active window.
    /// Called by the compositor when MotionWave routes Command+Arrow events.
    pub fn apply_tile_command(&mut self, window_id: u64, command: TileCommand) {
        let usable_y = self.panel_height as i32;
        let usable_height = self.display_height - self.panel_height - self.dock_height;
        let half_width = self.display_width / 2;

        let record = self.windows.iter_mut().find(|w| w.window_id == window_id);
        let record = match record {
            Some(r) => r,
            None => return,
        };

        let current_rect = record.current_rect;
        let (new_state, target_rect) = match command {
            TileCommand::TileLeft => (
                TileState::LeftHalf,
                WindowRect::new(0, usable_y, half_width, usable_height),
            ),
            TileCommand::TileRight => (
                TileState::RightHalf,
                WindowRect::new(half_width as i32, usable_y, half_width, usable_height),
            ),
            TileCommand::Maximize => (
                TileState::Maximized,
                WindowRect::new(0, usable_y, self.display_width, usable_height),
            ),
            TileCommand::Restore => {
                let restore = record.floating_rect;
                (TileState::Floating, restore)
            }
        };

        // Save floating geometry before any tile operation
        if record.tile_state == TileState::Floating {
            record.floating_rect = current_rect;
        }

        record.tile_state = new_state;

        // Cancel any existing animation for this window
        self.animations.retain(|a| a.window_id != window_id);

        // Begin new tile animation
        self.animations.push(TileAnimation::new(
            window_id,
            current_rect,
            target_rect,
            new_state,
        ));
    }

    /// Advance all tile animations. Call every compositor frame.
    /// Returns a list of (window_id, current_rect) for the compositor to apply.
    pub fn tick(&mut self, delta_t: f64) -> Vec<(u64, WindowRect)> {
        let mut updates = Vec::new();

        for anim in self.animations.iter_mut() {
            let rect = anim.tick(delta_t);
            // Update the window's current_rect in our records
            if let Some(record) = self.windows.iter_mut().find(|w| w.window_id == anim.window_id) {
                record.current_rect = rect;
            }
            updates.push((anim.window_id, rect));
        }

        // Remove completed animations
        self.animations.retain(|a| !a.is_complete());

        updates
    }

    /// Update display geometry (called on resolution or scale changes)
    pub fn on_display_changed(&mut self, width: u32, height: u32) {
        self.display_width = width;
        self.display_height = height;
    }
}

/// Tile command dispatched by MotionWave for Command+Arrow events
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileCommand {
    TileLeft,
    TileRight,
    Maximize,
    Restore,
}

impl Default for TilingWindowManager {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_left() {
        let mut twm = TilingWindowManager::new(1920, 1080);
        twm.register_window(1, WindowRect::new(100, 100, 800, 600));
        twm.apply_tile_command(1, TileCommand::TileLeft);

        assert_eq!(twm.animations.len(), 1);
        let anim = &twm.animations[0];
        assert_eq!(anim.to_rect.x, 0);
        assert_eq!(anim.to_rect.width, 960);
        assert_eq!(anim.target_state, TileState::LeftHalf);
    }

    #[test]
    fn test_tile_right() {
        let mut twm = TilingWindowManager::new(1920, 1080);
        twm.register_window(2, WindowRect::new(100, 100, 800, 600));
        twm.apply_tile_command(2, TileCommand::TileRight);

        let anim = &twm.animations[0];
        assert_eq!(anim.to_rect.x, 960);
        assert_eq!(anim.to_rect.width, 960);
    }

    #[test]
    fn test_maximize() {
        let mut twm = TilingWindowManager::new(1920, 1080);
        twm.register_window(3, WindowRect::new(200, 200, 600, 400));
        twm.apply_tile_command(3, TileCommand::Maximize);

        let anim = &twm.animations[0];
        assert_eq!(anim.to_rect.x, 0);
        assert_eq!(anim.to_rect.width, 1920);
        assert_eq!(anim.target_state, TileState::Maximized);
    }

    #[test]
    fn test_restore_to_floating() {
        let mut twm = TilingWindowManager::new(1920, 1080);
        let original = WindowRect::new(300, 300, 700, 500);
        twm.register_window(4, original);

        twm.apply_tile_command(4, TileCommand::TileLeft);
        twm.apply_tile_command(4, TileCommand::Restore);

        let anim = &twm.animations[0];
        assert_eq!(anim.to_rect, original);
        assert_eq!(anim.target_state, TileState::Floating);
    }

    #[test]
    fn test_animation_tick_advances() {
        let mut twm = TilingWindowManager::new(1920, 1080);
        twm.register_window(5, WindowRect::new(0, 0, 800, 600));
        twm.apply_tile_command(5, TileCommand::Maximize);

        let updates = twm.tick(0.1);
        assert_eq!(updates.len(), 1);
        let (id, _rect) = updates[0];
        assert_eq!(id, 5);
    }

    #[test]
    fn test_animation_completes() {
        let mut twm = TilingWindowManager::new(1920, 1080);
        twm.register_window(6, WindowRect::new(0, 0, 500, 400));
        twm.apply_tile_command(6, TileCommand::TileRight);

        // Advance far past animation duration
        twm.tick(1.0);
        assert!(twm.animations.is_empty());
    }

    #[test]
    fn test_unregister_cleans_up() {
        let mut twm = TilingWindowManager::new(1920, 1080);
        twm.register_window(7, WindowRect::new(0, 0, 400, 300));
        twm.apply_tile_command(7, TileCommand::TileLeft);
        twm.unregister_window(7);
        assert!(twm.windows.is_empty());
        assert!(twm.animations.is_empty());
    }
}
