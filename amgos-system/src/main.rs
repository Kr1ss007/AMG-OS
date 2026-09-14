//! AMGOS Process 1 — System Session
//!
//! Owns:
//! - Hardware detection & driver monitoring
//! - Network credentials vault & connection management
//! - Power management (sleep, wake, hibernate, shutdown)
//! - EventBus (e-bus) internal pub/sub broker
//! - Audio-Video Manager (AVM) with PipeWire/WirePlumber & boot chime sequencer
//! - Astrophage continuous rolling diagnostic logger
//! - Security boundaries & signed permission state
//!
//! Hard rule: Never owns rendered UI. Never exposes credentials to Process 2.

pub mod astrophage;
pub mod avm;
pub mod ebus;
pub mod eobus;
pub mod hardware;
pub mod network;
pub mod permissions;
pub mod power;
pub mod supervisor;

use amgos_protocol::ebus::{AstrophageLevel, SystemEvent};
use astrophage::AstrophageCoreLogger;
use avm::AudioVideoManager;
use ebus::{EventBusServer, DEFAULT_EBUS_SOCKET_PATH};
use hardware::detect_hardware;
use network::NetworkCredentialVault;
use permissions::PermissionManager;
use power::PowerManager;
use supervisor::ServiceSupervisor;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[amgos-system] Starting Process 1 (System Session)...");

    // 1. Initialize Astrophage Continuous Event Logger
    let astrophage = AstrophageCoreLogger::new();
    astrophage.log(
        AstrophageLevel::Info,
        "system-init",
        "Process 1 initialization sequence started",
        None,
    );

    // 2. Hardware Detection
    let hw_profile = detect_hardware();
    println!(
        "[amgos-system] Hardware detected: CPU: {} ({} cores), RAM: {} MB, GPU: {}",
        hw_profile.cpu_model,
        hw_profile.cpu_cores,
        hw_profile.total_memory_bytes / (1024 * 1024),
        hw_profile.discrete_gpu_detected
    );

    astrophage.log(
        AstrophageLevel::Info,
        "hardware",
        &format!("CPU detected: {}", hw_profile.cpu_model),
        Some(("cpu_cores", hw_profile.cpu_cores as f64)),
    );

    // 3. Audio-Video Manager (AVM)
    let avm = AudioVideoManager::new();
    avm.mark_audio_hardware_ready();

    // 4. Network Vault & Power Manager & Permission Manager
    let _network = NetworkCredentialVault::new();
    let _power = PowerManager::new();
    let _permissions = PermissionManager::new();

    // 5. Bind internal pub/sub EventBus (e-bus)
    let ebus_server = Arc::new(
        EventBusServer::bind(DEFAULT_EBUS_SOCKET_PATH)
            .expect("Failed to bind EventBus socket at /tmp/amgos-ebus.sock"),
    );
    println!(
        "[amgos-system] EventBus (e-bus) listening on {}",
        DEFAULT_EBUS_SOCKET_PATH
    );

    // 6. Start Supervisor and worker tasks
    let mut supervisor = ServiceSupervisor::new();
    let ebus_clone = Arc::clone(&ebus_server);

    supervisor.register_service("ebus-poller", move |_running| {
        let _ = ebus_clone.poll_connections();
    });

    // 7. Publish HardwareReady event over e-bus
    let hw_event = SystemEvent::HardwareReady {
        timestamp_ns: now_ns(),
        cpu_model: hw_profile.cpu_model,
        cpu_cores: hw_profile.cpu_cores,
        total_memory_bytes: hw_profile.total_memory_bytes,
        discrete_gpu: hw_profile.discrete_gpu_detected,
    };
    let _ = ebus_server.publish_event(&hw_event);

    // 8. Publish AudioReady and trigger Boot Chime
    let audio_event = SystemEvent::AudioReady {
        timestamp_ns: now_ns(),
        sample_rate: avm::AUDIO_SAMPLE_RATE,
        channels: avm::AUDIO_CHANNELS,
    };
    let _ = ebus_server.publish_event(&audio_event);

    if let Some(chime) = avm.evaluate_chime_sequencer() {
        println!(
            "[amgos-system] AVM Chime Sequencer triggered: Note {} ({:.2} Hz, duration {} ms)",
            chime.note, chime.frequency_hz, chime.duration_ms
        );
        let chime_event = SystemEvent::BootChimeTrigger {
            timestamp_ns: now_ns(),
            note: chime.note,
            frequency_hz: chime.frequency_hz,
            duration_ms: chime.duration_ms,
        };
        let _ = ebus_server.publish_event(&chime_event);
    }

    // 9. Keep Process 1 running cleanly
    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);
    let _ = ctrlc_handler(move || {
        println!("\n[amgos-system] Controlled shutdown requested.");
        r.store(false, Ordering::SeqCst);
    });

    println!("[amgos-system] Process 1 operational and ready.");
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
        let _ = ebus_server.poll_connections();
    }

    supervisor.shutdown();
    ebus_server.shutdown();
    println!("[amgos-system] Process 1 shutdown complete.");
    Ok(())
}

fn ctrlc_handler<F>(f: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() + Send + 'static,
{
    // Basic signal handling
    thread::spawn(move || {
        // Can be replaced with signalfd/nix in full production build
        let _ = f;
    });
    Ok(())
}
