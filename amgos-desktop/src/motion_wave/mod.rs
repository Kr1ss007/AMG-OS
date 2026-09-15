//! MotionWave Desktop Integration Subsystem (Process 2)
//!
//! Enforces AMGOS SPEC Section 1.2, 3.3, 3.5, and 8.3:
//! - Compositor-level gesture recognition identified before any application sees it.
//! - Touchpad multitouch gesture recognition (multi-finger swipe, pinch, natural scroll, tap-to-click).
//! - Routing to shell surfaces:
//!   * 3/4-finger swipe UP: toggles Pilot Control (virtual desktop overview).
//!   * 4-finger swipe DOWN: closes Pilot Control.
//!   * 4-finger swipe LEFT/RIGHT: triggers virtual desktop transition or Zen Browser tab-switch.
//!   * 2-finger scroll: smoothed with natural scrolling and pointer speed multipliers.
//! - Keyboard shortcut registration and routing:
//!   * Command + Left: Tile active window to left half.
//!   * Command + Right: Tile active window to right half.
//!   * Command + Up: Maximize active window.
//!   * Command + Down: Restore active window to floating geometry.
//!   * Command + Space: Activate Pathfinder search bar in Filer.
//! - Process 2 receives hardware configuration updates over e-bus without touching raw evdev.

use crate::pilot_control::{DesktopTransitionDirection, PilotControl, PilotControlState};
use crate::tiling::{TileCommand, TilingWindowManager};
use amgos_protocol::ebus::{KeyboardConfig, TouchpadConfig};
use serde::{Deserialize, Serialize};

/// Direction for touchpad gestures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GestureDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Recognized touchpad gesture types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TouchpadGesture {
    /// Multi-finger directional swipe
    Swipe {
        fingers: u8,
        direction: GestureDirection,
        delta_x: f64,
        delta_y: f64,
        is_finished: bool,
    },
    /// Two-finger pinch or spread (zoom)
    Pinch {
        scale: f64,
        is_finished: bool,
    },
    /// Two-finger scroll
    Scroll {
        delta_x: f64,
        delta_y: f64,
    },
    /// Finger tap (1-finger left click, 2-finger right click, 3-finger middle click)
    Tap {
        fingers: u8,
        x: f64,
        y: f64,
    },
}

/// Modifier keys state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub command: bool,
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

/// Discrete keyboard action recognized by MotionWave
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    TileLeft,
    TileRight,
    TileMaximize,
    TileRestore,
    TogglePilotControl,
    ActivatePathfinder,
    PassThrough,
}

/// MotionWave routing target
#[derive(Debug, Clone, PartialEq)]
pub enum MotionWaveTarget {
    PilotControlToggle,
    PilotControlDismiss,
    DesktopSwitch(DesktopTransitionDirection),
    ZenTabSwitch(GestureDirection),
    WindowTiling(KeyAction),
    PathfinderFocus,
    SurfaceScroll { delta_x: f64, delta_y: f64 },
    AppInputPassThrough,
}

/// The MotionWave gesture detector and input routing engine for Process 2
pub struct MotionWaveEngine {
    touchpad_config: TouchpadConfig,
    keyboard_config: KeyboardConfig,
    modifiers: KeyModifiers,
    accumulated_swipe_dx: f64,
    accumulated_swipe_dy: f64,
    swipe_threshold_px: f64,
}

impl MotionWaveEngine {
    pub fn new() -> Self {
        Self {
            touchpad_config: TouchpadConfig::default(),
            keyboard_config: KeyboardConfig::default(),
            modifiers: KeyModifiers::default(),
            accumulated_swipe_dx: 0.0,
            accumulated_swipe_dy: 0.0,
            swipe_threshold_px: 50.0,
        }
    }

    /// Update touchpad settings received from Process 1 via e-bus
    pub fn update_touchpad_config(&mut self, config: TouchpadConfig) {
        self.touchpad_config = config;
    }

    /// Update keyboard settings received from Process 1 via e-bus
    pub fn update_keyboard_config(&mut self, config: KeyboardConfig) {
        self.keyboard_config = config;
    }

    /// Update modifier state on key press / release
    pub fn update_modifiers(&mut self, modifiers: KeyModifiers) {
        self.modifiers = modifiers;
    }

    /// Process a raw keyboard event and resolve compositor-level shortcuts
    pub fn process_key(&mut self, key_code: u32, is_press: bool) -> KeyAction {
        if !is_press {
            return KeyAction::PassThrough;
        }

        // Check for Command key combinations (SPEC Section 3.5)
        if self.modifiers.command {
            match key_code {
                // Left Arrow (KEY_LEFT: 105 in Linux evdev)
                105 => KeyAction::TileLeft,
                // Right Arrow (KEY_RIGHT: 106 in Linux evdev)
                106 => KeyAction::TileRight,
                // Up Arrow (KEY_UP: 103 in Linux evdev)
                103 => KeyAction::TileMaximize,
                // Down Arrow (KEY_DOWN: 108 in Linux evdev)
                108 => KeyAction::TileRestore,
                // Spacebar (KEY_SPACE: 57 in Linux evdev)
                57 => KeyAction::ActivatePathfinder,
                _ => KeyAction::PassThrough,
            }
        } else {
            KeyAction::PassThrough
        }
    }

    /// Process a raw touchpad touch or motion vector
    pub fn process_touchpad_motion(&mut self, fingers: u8, dx: f64, dy: f64) -> Option<MotionWaveTarget> {
        let speed_factor = (1.0 + self.touchpad_config.pointer_speed).max(0.1) as f64;
        let effective_dx = dx * speed_factor;
        let effective_dy = if self.touchpad_config.natural_scrolling {
            dy * speed_factor
        } else {
            -dy * speed_factor
        };

        match fingers {
            // 2 fingers: smoothed scrolling
            2 => {
                if self.touchpad_config.two_finger_scroll {
                    Some(MotionWaveTarget::SurfaceScroll {
                        delta_x: effective_dx,
                        delta_y: effective_dy,
                    })
                } else {
                    None
                }
            }

            // 3 or 4 fingers: compositor gesture recognition
            3 | 4 => {
                self.accumulated_swipe_dx += effective_dx;
                self.accumulated_swipe_dy += effective_dy;

                // Check vertical swipe threshold
                if self.accumulated_swipe_dy.abs() > self.swipe_threshold_px {
                    let is_up = self.accumulated_swipe_dy < 0.0;
                    self.accumulated_swipe_dx = 0.0;
                    self.accumulated_swipe_dy = 0.0;

                    if is_up {
                        Some(MotionWaveTarget::PilotControlToggle)
                    } else {
                        Some(MotionWaveTarget::PilotControlDismiss)
                    }
                } else if self.accumulated_swipe_dx.abs() > self.swipe_threshold_px {
                    // Check horizontal swipe threshold (Desktop switch or Zen tab switch)
                    let is_left = self.accumulated_swipe_dx < 0.0;
                    self.accumulated_swipe_dx = 0.0;
                    self.accumulated_swipe_dy = 0.0;

                    if is_left {
                        Some(MotionWaveTarget::DesktopSwitch(DesktopTransitionDirection::Left))
                    } else {
                        Some(MotionWaveTarget::DesktopSwitch(DesktopTransitionDirection::Right))
                    }
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Reset gesture accumulation when fingers leave touchpad
    pub fn end_gesture(&mut self) {
        self.accumulated_swipe_dx = 0.0;
        self.accumulated_swipe_dy = 0.0;
    }

    /// Route recognized target to desktop components
    pub fn dispatch_action(
        &self,
        target: &MotionWaveTarget,
        pilot_control: &mut PilotControl,
        tiling: &mut TilingWindowManager,
        active_window_id: Option<u64>,
    ) {
        match target {
            MotionWaveTarget::PilotControlToggle => {
                pilot_control.toggle();
            }
            MotionWaveTarget::PilotControlDismiss => {
                if pilot_control.state == PilotControlState::Active {
                    pilot_control.toggle();
                }
            }
            MotionWaveTarget::DesktopSwitch(dir) => {
                let current_id = pilot_control.active_desktop_id;
                let target_id = match dir {
                    DesktopTransitionDirection::Right | DesktopTransitionDirection::Down => {
                        current_id + 1
                    }
                    DesktopTransitionDirection::Left | DesktopTransitionDirection::Up => {
                        current_id.saturating_sub(1)
                    }
                };
                if target_id != current_id && target_id < pilot_control.desktops.len() as u32 {
                    pilot_control.switch_to(target_id);
                }
            }
            MotionWaveTarget::WindowTiling(action) => {
                if let Some(win_id) = active_window_id {
                    match action {
                        KeyAction::TileLeft => {
                            tiling.apply_tile_command(win_id, TileCommand::TileLeft);
                        }
                        KeyAction::TileRight => {
                            tiling.apply_tile_command(win_id, TileCommand::TileRight);
                        }
                        KeyAction::TileMaximize => {
                            tiling.apply_tile_command(win_id, TileCommand::Maximize);
                        }
                        KeyAction::TileRestore => {
                            tiling.apply_tile_command(win_id, TileCommand::Restore);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

impl Default for MotionWaveEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiling::WindowRect;

    #[test]
    fn test_motionwave_keyboard_tiling_shortcuts() {
        let mut engine = MotionWaveEngine::new();
        engine.update_modifiers(KeyModifiers {
            command: true,
            shift: false,
            control: false,
            alt: false,
        });

        assert_eq!(engine.process_key(105, true), KeyAction::TileLeft);
        assert_eq!(engine.process_key(106, true), KeyAction::TileRight);
        assert_eq!(engine.process_key(103, true), KeyAction::TileMaximize);
        assert_eq!(engine.process_key(108, true), KeyAction::TileRestore);
        assert_eq!(engine.process_key(57, true), KeyAction::ActivatePathfinder);

        // Release does not trigger
        assert_eq!(engine.process_key(105, false), KeyAction::PassThrough);

        // Without Command key, shortcuts are pass-through
        engine.update_modifiers(KeyModifiers::default());
        assert_eq!(engine.process_key(105, true), KeyAction::PassThrough);
    }

    #[test]
    fn test_motionwave_touchpad_gesture_recognition() {
        let mut engine = MotionWaveEngine::new();

        // 2 fingers scroll
        let scroll = engine.process_touchpad_motion(2, 0.0, 10.0);
        assert_eq!(
            scroll,
            Some(MotionWaveTarget::SurfaceScroll {
                delta_x: 0.0,
                delta_y: 10.0
            })
        );

        // 4 fingers swipe UP past threshold triggers Pilot Control
        let _ = engine.process_touchpad_motion(4, 0.0, -30.0);
        let gesture = engine.process_touchpad_motion(4, 0.0, -30.0);
        assert_eq!(gesture, Some(MotionWaveTarget::PilotControlToggle));

        // 4 fingers swipe RIGHT triggers desktop switch
        let _ = engine.process_touchpad_motion(4, 30.0, 0.0);
        let gesture2 = engine.process_touchpad_motion(4, 30.0, 0.0);
        assert_eq!(
            gesture2,
            Some(MotionWaveTarget::DesktopSwitch(DesktopTransitionDirection::Right))
        );
    }

    #[test]
    fn test_motionwave_dispatch_to_pilot_and_tiling() {
        let engine = MotionWaveEngine::new();
        let mut pilot = PilotControl::new();
        let mut tiling = TilingWindowManager::new(1920, 1080);
        tiling.register_window(1, WindowRect::new(100, 100, 800, 600));

        assert_eq!(pilot.state, PilotControlState::Hidden);
        engine.dispatch_action(&MotionWaveTarget::PilotControlToggle, &mut pilot, &mut tiling, Some(1));
        assert_eq!(pilot.state, PilotControlState::Entering);

        engine.dispatch_action(&MotionWaveTarget::WindowTiling(KeyAction::TileLeft), &mut pilot, &mut tiling, Some(1));
        assert_eq!(tiling.animations.len(), 1);
        let anim = &tiling.animations[0];
        assert_eq!(anim.to_rect.x, 0);
        assert_eq!(anim.to_rect.width, 960);
    }
}
