//! AMGOS Process 2 — Desktop Session
//!
//! Owns:
//! - Wayland compositor interface
//! - All rendered UI surfaces
//! - Animation and Interface Engine (cubic-bezier timing system)
//! - Top Global Menu Bar (Space Orange identity mark, full width)
//! - Window Chrome Traffic Lights
//! - Lockscreen
//! - Setup Wizard
//! - Shutdown & Restart UI ("goodbye", "i'll see you in a bit")
//! - EventBus (e-bus) subscriber client
//!
//! Hard rule: Never touches hardware directly. Never holds network credentials.
//! Requests actions exclusively across EventBus to Process 1.

pub mod animations;
pub mod compositor;
pub mod ebus;
pub mod eobus;
pub mod lockscreen;
pub mod shutdown_ui;
pub mod topbar;
pub mod traffic_lights;
pub mod wizard;

use amgos_protocol::ebus::{DesktopRequest, EventBusClient, SystemEvent, DEFAULT_EBUS_SOCKET_PATH};
use compositor::CompositorBridge;
use lockscreen::LockscreenState;
use topbar::TopGlobalMenuBar;
use traffic_lights::TrafficLightGroup;
use wizard::SetupWizardState;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[amgos-desktop] Starting Process 2 (Desktop Session)...");

    let _compositor = CompositorBridge::new();
    let mut topbar = TopGlobalMenuBar::new();
    let mut traffic_lights = TrafficLightGroup::new();
    let _lockscreen = LockscreenState::new("AMG-OS User");
    let mut _wizard = SetupWizardState::new();

    println!(
        "[amgos-desktop] Topbar initialized: Identity Mark [{}], App: {}",
        topbar.identity_mark.color_hex, topbar.active_app_title
    );

    traffic_lights.set_hover(true);
    println!(
        "[amgos-desktop] Window chrome traffic lights: Close: {:?}, Min: {:?}, Max: {:?}",
        traffic_lights.close_symbol(),
        traffic_lights.minimize_symbol(),
        traffic_lights.maximize_symbol()
    );

    // Attempt connecting to Process 1 EventBus (e-bus)
    println!(
        "[amgos-desktop] Connecting to EventBus at {}...",
        DEFAULT_EBUS_SOCKET_PATH
    );
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = Arc::clone(&running);

    if let Ok(client) = EventBusClient::connect(DEFAULT_EBUS_SOCKET_PATH) {
        println!("[amgos-desktop] Connected to Process 1 EventBus successfully.");

        // Query Pathfinder as a test request
        let req = DesktopRequest::SearchPathfinder {
            query_id: 1,
            query: "welcome".to_string(),
            max_results: 10,
        };
        if let Ok(msg_id) = client.send_request(&req) {
            println!(
                "[amgos-desktop] Dispatched DesktopRequest #{} over e-bus.",
                msg_id
            );
        }

        // Spawn background listener thread for SystemEvents
        thread::spawn(move || {
            while r_clone.load(Ordering::SeqCst) {
                match client.read_event() {
                    Ok(event) => match event {
                        SystemEvent::HardwareReady {
                            cpu_model,
                            discrete_gpu,
                            ..
                        } => {
                            println!(
                                "[amgos-desktop] Received HardwareReady: CPU: {}, GPU: {}",
                                cpu_model, discrete_gpu
                            );
                        }
                        SystemEvent::AudioReady { sample_rate, .. } => {
                            println!("[amgos-desktop] Received AudioReady: {} Hz", sample_rate);
                        }
                        SystemEvent::BootChimeTrigger {
                            note, frequency_hz, ..
                        } => {
                            println!(
                                "[amgos-desktop] Received BootChimeTrigger: Note {} ({:.2} Hz)",
                                note, frequency_hz
                            );
                        }
                        SystemEvent::NetworkStateChanged {
                            connected, ssid, ..
                        } => {
                            println!(
                                "[amgos-desktop] Network changed: Connected: {}, SSID: {:?}",
                                connected, ssid
                            );
                        }
                        _ => {}
                    },
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });
    } else {
        println!("[amgos-desktop] Process 1 EventBus offline (will retry during boot sequence).");
    }

    // Main desktop rendering & frame loop simulation
    topbar.update_network_status(true);
    println!("[amgos-desktop] Desktop Session active. Compositor ready.");

    // Loop briefly or stay alive
    thread::sleep(Duration::from_millis(500));

    running.store(false, Ordering::SeqCst);
    println!("[amgos-desktop] Desktop Session terminated cleanly.");
    Ok(())
}
