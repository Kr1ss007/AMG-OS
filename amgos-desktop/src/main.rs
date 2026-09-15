//! AMGOS Process 2 — Desktop Session
//!
//! Owns:
//! - Pure Wayland Display Server & Compositor (Process 2)
//! - Framebuffer continuity gate: holds GOP framebuffer without blanking
//! - Top Global Menu Bar with Space Orange identity mark (#FF5500, 36x24px)
//! - Smart Dock with auto-hide and cubic-bezier animations
//! - Pilot Control (Mission Control adjacent virtual desktop manager)
//! - MotionWave gesture recognition and input routing
//! - Tiling window manager (Command + Arrow keys)
//! - Notification Center & FreeDesktop StatusNotifierItem (SNI) host
//! - System screens: Lockscreen, First-Boot Setup Wizard, Shutdown ("goodbye"), Restart
//!
//! Hard rule: Never touches hardware directly. Never holds network credentials.

pub mod animations;
pub mod compositor;
pub mod lockscreen;
pub mod motion_wave;
pub mod notifications;
pub mod pilot_control;
pub mod shutdown_ui;
pub mod smart_dock;
pub mod tiling;
pub mod topbar;
pub mod traffic_lights;
pub mod wizard;

use amgos_protocol::ebus::{
    DesktopRequest, EventBusClient, SystemEvent, DEFAULT_EBUS_SOCKET_PATH,
};
use compositor::CompositorBridge;
use crossbeam_channel::unbounded;
use lockscreen::LockscreenState;
use motion_wave::MotionWaveEngine;
use notifications::NotificationCenter;
use pilot_control::PilotControl;
use shutdown_ui::{PowerScreenState, PowerScreenType};
use smart_dock::{DockPosition, SmartDock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tiling::TilingWindowManager;
use topbar::TopGlobalMenuBar;
use traffic_lights::TrafficLightGroup;
use wizard::SetupWizardState;

#[derive(Debug, Clone, Copy, PartialEq)]
enum BootPhase {
    Loading,
    ChimeWordmarkSync,
    TransitionToDesktop,
    DesktopReady,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Note: Zero TTY output in Process 2 per spec Section 4.1.
    // The compositor owns all boot display.

    let mut compositor = CompositorBridge::new();
    let mut topbar = TopGlobalMenuBar::new();
    let _traffic_lights = TrafficLightGroup::new();
    let mut lockscreen = LockscreenState::new("AMG-OS User");
    let wizard = SetupWizardState::new();
    let shutdown_screen = PowerScreenState::new(PowerScreenType::Shutdown);
    let restart_screen = PowerScreenState::new(PowerScreenType::Restart);
    let mut active_power_type: Option<PowerScreenType> = None;
    let mut dock = SmartDock::new(DockPosition::Bottom);
    let mut pilot_control = PilotControl::new();
    let mut tiling = TilingWindowManager::new(compositor.display_width(), compositor.display_height());
    let mut notifications = NotificationCenter::new();
    let mut motion_wave = MotionWaveEngine::new();

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = Arc::clone(&running);
    let (event_tx, event_rx) = unbounded();

    // EventBus background client for Process 1 pub/sub
    if let Ok(client) = EventBusClient::connect(DEFAULT_EBUS_SOCKET_PATH) {
        let scan_req = DesktopRequest::ScanWifiAccessPoints;
        let _ = client.send_request(&scan_req);

        thread::spawn(move || {
            while r_clone.load(Ordering::SeqCst) {
                if let Ok(event) = client.read_event() {
                    let _ = event_tx.send(event);
                } else {
                    thread::sleep(Duration::from_millis(50));
                }
            }
        });
    }

    // --- Compositor frame loop ---
    let frame_delta = 1.0 / 144.0;

    // Boot Phase State Machine
    let mut boot_phase = BootPhase::Loading;
    let mut loading_progress: f64 = 0.0;
    let mut wordmark_opacity: f64 = 0.0;
    let is_first_boot = true;

    topbar.update_network_status(true);

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(7)); // ~144Hz loop target

        // Process incoming events from Process 1
        while let Ok(event) = event_rx.try_recv() {
            topbar.handle_system_event(&event);
            compositor.handle_system_event(&event);

            match event {
                SystemEvent::BootChimeTrigger { .. } => {
                    // Sync wordmark reveal with the hardware chime trigger
                    if boot_phase == BootPhase::Loading {
                        boot_phase = BootPhase::ChimeWordmarkSync;
                        loading_progress = 1.0;
                    }
                }
                SystemEvent::NotificationDispatched {
                    notification_id,
                    app_id,
                    title,
                    body,
                    urgency,
                } => {
                    notifications.push(notification_id, &app_id, &title, &body, urgency);
                }
                SystemEvent::TouchpadConfigChanged(cfg) => {
                    motion_wave.update_touchpad_config(cfg);
                }
                SystemEvent::KeyboardConfigChanged(cfg) => {
                    motion_wave.update_keyboard_config(cfg);
                }
                SystemEvent::PowerStateTransition { target, .. } => {
                    match target {
                        amgos_protocol::ebus::PowerState::Shutdown => {
                            active_power_type = Some(PowerScreenType::Shutdown);
                        }
                        amgos_protocol::ebus::PowerState::Reboot => {
                            active_power_type = Some(PowerScreenType::Restart);
                        }
                        _ => {}
                    }
                }
                SystemEvent::FirstBootConfigApplied { .. } => {
                    // Setup Wizard completed, ready desktop
                    boot_phase = BootPhase::DesktopReady;
                }
                _ => {}
            }
        }

        // Advance animation & state machines
        match boot_phase {
            BootPhase::Loading => {
                // Render macOS-style progress bar filling up
                loading_progress = (loading_progress + frame_delta * 0.4).min(0.95);
            }
            BootPhase::ChimeWordmarkSync => {
                // Fade in AMG-OS wordmark
                wordmark_opacity = (wordmark_opacity + frame_delta * 3.0).min(1.0);
                if wordmark_opacity >= 1.0 {
                    boot_phase = BootPhase::TransitionToDesktop;
                }
            }
            BootPhase::TransitionToDesktop => {
                if is_first_boot && !wizard.is_completed {
                    boot_phase = BootPhase::DesktopReady;
                } else {
                    lockscreen.unlock();
                    boot_phase = BootPhase::DesktopReady;
                }
            }
            BootPhase::DesktopReady => {
                // Tick standard desktop subsystems
                let _ = dock.tick(frame_delta);
                let _ = pilot_control.tick(frame_delta);
                let _ = notifications.tick(frame_delta);
                let _ = tiling.tick(frame_delta);
                let _ = lockscreen.tick(frame_delta);

                let active_screen = match active_power_type {
                    Some(PowerScreenType::Shutdown) => Some(&shutdown_screen),
                    Some(PowerScreenType::Restart) => Some(&restart_screen),
                    None => None,
                };

                // Render full layered UI frame to compositor
                compositor.render_frame(
                    &topbar,
                    &dock,
                    &pilot_control,
                    &tiling,
                    &notifications,
                    if is_first_boot && !wizard.is_completed { Some(&wizard) } else { None },
                    active_screen,
                );

                compositor.tick_frame();
            }
        }
    }

    Ok(())
}
