//! Smart Dock (Process 2 — Animation and Interface Engine)
//!
//! The Smart Dock auto-hides when a window moves over or near it.
//! Auto-shows when the cursor approaches the dock edge or no window is near it.
//! Never obscures active content. Never requires the user to manage it manually.
//!
//! Contents: Pinned apps, running apps (indicated clearly), recent apps.
//! Animation: cubic-bezier timing system governs show/hide transitions.
//! Input: MotionWave gesture and cursor proximity events trigger show/hide.
//!
//! Per spec Section 3.2: Dock lives at Layer 5 (above windows when visible).

use crate::animations::curves;
use std::collections::VecDeque;

/// Maximum number of recent apps to track
pub const MAX_RECENT_APPS: usize = 6;

/// Cursor proximity threshold from dock edge that triggers reveal (pixels)
pub const DOCK_REVEAL_THRESHOLD_PX: i32 = 4;

/// Dock geometry (bottom-centered by default, matches macOS aesthetic)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockPosition {
    Bottom,
    Left,
    Right,
}

/// One entry in the dock — either a pinned app, a running app, or a recent app
#[derive(Debug, Clone, PartialEq)]
pub struct DockEntry {
    pub app_id: String,
    pub display_name: String,
    pub icon_path: Option<String>,
    pub is_running: bool,
    pub is_pinned: bool,
    /// Number of windows open for this app (>1 shows stacked indicator)
    pub window_count: u32,
}

/// Animated visibility state driven by cubic-bezier solver
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockVisibility {
    Shown,
    Hidden,
    AnimatingIn,
    AnimatingOut,
}

pub struct SmartDock {
    pub position: DockPosition,
    pub visibility: DockVisibility,
    pub entries: Vec<DockEntry>,
    pub recent_apps: VecDeque<DockEntry>,

    /// Current animation progress [0.0, 1.0]
    animation_progress: f64,
    /// True if any window is currently within the dock's proximity zone
    window_overlapping: bool,
    /// True if the cursor is within DOCK_REVEAL_THRESHOLD_PX of the dock edge
    cursor_near_edge: bool,
}

impl SmartDock {
    pub fn new(position: DockPosition) -> Self {
        let mut dock = Self {
            position,
            visibility: DockVisibility::Shown,
            entries: Vec::new(),
            recent_apps: VecDeque::with_capacity(MAX_RECENT_APPS),
            animation_progress: 1.0,
            window_overlapping: false,
            cursor_near_edge: false,
        };

        // Seed the dock with the system-defined first-class native apps
        dock.pin_app("filer", "Filer", None);
        dock.pin_app("zen-browser", "Zen Browser", None);
        dock.pin_app("terminow", "Terminow", None);
        dock.pin_app("settings", "Settings", None);
        dock.pin_app("astrophage", "Astrophage", None);

        dock
    }

    /// Pin a native citizen app to the dock at initialization
    fn pin_app(&mut self, app_id: &str, display_name: &str, icon_path: Option<&str>) {
        self.entries.push(DockEntry {
            app_id: app_id.to_string(),
            display_name: display_name.to_string(),
            icon_path: icon_path.map(str::to_string),
            is_running: false,
            is_pinned: true,
            window_count: 0,
        });
    }

    /// Mark an app as launched — shows running indicator in dock
    pub fn on_app_launched(&mut self, app_id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.app_id == app_id) {
            entry.is_running = true;
            entry.window_count += 1;
        } else {
            // Non-pinned running app — add it dynamically
            self.entries.push(DockEntry {
                app_id: app_id.to_string(),
                display_name: app_id.to_string(),
                icon_path: None,
                is_running: true,
                is_pinned: false,
                window_count: 1,
            });
        }
    }

    /// Mark an app as terminated — removes running indicator
    pub fn on_app_closed(&mut self, app_id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.app_id == app_id) {
            entry.window_count = entry.window_count.saturating_sub(1);
            if entry.window_count == 0 {
                entry.is_running = false;
                // Add to recents if not pinned
                if !entry.is_pinned {
                    let recent = entry.clone();
                    // Remove the non-pinned dynamic entry
                    self.entries.retain(|e| e.app_id != app_id);
                    // Push to recents, evict oldest if full
                    if self.recent_apps.len() >= MAX_RECENT_APPS {
                        self.recent_apps.pop_back();
                    }
                    self.recent_apps.push_front(recent);
                }
            }
        }
    }

    /// Called by the compositor when a window enters the dock's proximity zone.
    /// Begins the animated hide sequence using the ACCELERATE curve.
    pub fn on_window_overlapping(&mut self, overlapping: bool) {
        if overlapping == self.window_overlapping {
            return;
        }
        self.window_overlapping = overlapping;
        self.evaluate_visibility();
    }

    /// Called by MotionWave when the cursor enters/leaves the dock reveal zone.
    pub fn on_cursor_near_edge(&mut self, near: bool) {
        if near == self.cursor_near_edge {
            return;
        }
        self.cursor_near_edge = near;
        self.evaluate_visibility();
    }

    fn evaluate_visibility(&mut self) {
        let should_show = self.cursor_near_edge || !self.window_overlapping;
        match (should_show, self.visibility) {
            (true, DockVisibility::Hidden | DockVisibility::AnimatingOut) => {
                self.visibility = DockVisibility::AnimatingIn;
                self.animation_progress = 0.0;
            }
            (false, DockVisibility::Shown | DockVisibility::AnimatingIn) => {
                self.visibility = DockVisibility::AnimatingOut;
                self.animation_progress = 1.0;
            }
            _ => {}
        }
    }

    /// Advance the dock animation by `delta_t` seconds (called every compositor frame).
    /// Returns the current visual offset (0.0 = fully shown, 1.0 = fully hidden).
    pub fn tick(&mut self, delta_t: f64) -> f64 {
        let animation_duration = 0.22; // 220ms, per spec intent

        match self.visibility {
            DockVisibility::AnimatingIn => {
                self.animation_progress =
                    (self.animation_progress + delta_t / animation_duration).min(1.0);
                if self.animation_progress >= 1.0 {
                    self.visibility = DockVisibility::Shown;
                }
                // DECELERATE curve: dock eases in smoothly
                1.0 - curves::DECELERATE.solve(self.animation_progress)
            }
            DockVisibility::AnimatingOut => {
                self.animation_progress =
                    (self.animation_progress - delta_t / animation_duration).max(0.0);
                if self.animation_progress <= 0.0 {
                    self.visibility = DockVisibility::Hidden;
                }
                // ACCELERATE curve: dock accelerates out of view
                1.0 - curves::ACCELERATE.solve(self.animation_progress)
            }
            DockVisibility::Shown => 0.0,   // No offset — fully visible
            DockVisibility::Hidden => 1.0,  // Fully hidden
        }
    }

    pub fn is_visible(&self) -> bool {
        !matches!(self.visibility, DockVisibility::Hidden)
    }
}

impl Default for SmartDock {
    fn default() -> Self {
        Self::new(DockPosition::Bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_initializes_with_native_apps() {
        let dock = SmartDock::default();
        assert_eq!(dock.entries.len(), 5);
        assert!(dock.entries.iter().all(|e| e.is_pinned));
        assert_eq!(dock.visibility, DockVisibility::Shown);
    }

    #[test]
    fn test_dock_hides_on_window_overlap() {
        let mut dock = SmartDock::default();
        dock.on_window_overlapping(true);
        assert_eq!(dock.visibility, DockVisibility::AnimatingOut);

        dock.on_window_overlapping(false);
        assert_eq!(dock.visibility, DockVisibility::AnimatingIn);
    }

    #[test]
    fn test_dock_shows_on_cursor_near_edge() {
        let mut dock = SmartDock::default();
        // Force dock to hidden
        dock.window_overlapping = true;
        dock.visibility = DockVisibility::Hidden;
        dock.animation_progress = 0.0;

        dock.on_cursor_near_edge(true);
        assert_eq!(dock.visibility, DockVisibility::AnimatingIn);
    }

    #[test]
    fn test_app_launch_and_close() {
        let mut dock = SmartDock::default();
        dock.on_app_launched("filer");
        let filer = dock.entries.iter().find(|e| e.app_id == "filer").unwrap();
        assert!(filer.is_running);
        assert_eq!(filer.window_count, 1);

        dock.on_app_closed("filer");
        let filer = dock.entries.iter().find(|e| e.app_id == "filer").unwrap();
        // Pinned app stays; running flag cleared
        assert!(!filer.is_running);
    }

    #[test]
    fn test_recent_apps_for_non_pinned() {
        let mut dock = SmartDock::default();
        dock.on_app_launched("some-external-app");
        dock.on_app_closed("some-external-app");
        assert_eq!(dock.recent_apps.len(), 1);
        assert_eq!(dock.recent_apps[0].app_id, "some-external-app");
    }

    #[test]
    fn test_animation_tick() {
        let mut dock = SmartDock::default();
        dock.visibility = DockVisibility::AnimatingOut;
        dock.animation_progress = 1.0;

        // Advance 500ms — should complete the 220ms animation
        let offset = dock.tick(0.5);
        assert!(offset >= 0.0 && offset <= 1.0);
        assert_eq!(dock.visibility, DockVisibility::Hidden);
    }
}
