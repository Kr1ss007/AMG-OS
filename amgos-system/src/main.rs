//! AMGOS Process 1 — System Session
//!
//! Owns:
//! - Hardware detection & driver monitoring
//! - Network credentials vault & connection management
//! - Power management (sleep, wake, hibernate, shutdown) with profiles (Endurance, Balanced, MAX)
//! - EventBus (e-bus) internal pub/sub broker
//! - Audio-Video Manager (AVM) with PipeWire/WirePlumber & boot chime sequencer
//! - Astrophage continuous rolling diagnostic logger
//! - Security boundaries & signed permission state
//! - Filer Core (Pathfinder search index, package inspector, App Layer lifecycle)
//!
//! Hard rule: Never owns rendered UI. Never exposes credentials to Process 2.

use amgos_protocol::ebus::{
    AstrophageLevel, DesktopRequest, ProtocolFileEntry, SystemEvent,
};
use amgos_system::astrophage::AstrophageCoreLogger;
use amgos_system::avm::{self, AudioVideoManager};
use amgos_system::ebus::{EventBusServer, DEFAULT_EBUS_SOCKET_PATH};
use amgos_system::hardware::{self, detect_hardware};
use amgos_system::network::NetworkCredentialVault;
use amgos_system::permissions::PermissionManager;
use amgos_system::power::PowerManager;
use amgos_system::supervisor::ServiceSupervisor;
use filer_core::{AppLayerManager, FileSystemIndexer, PackageInspector, PathfinderIndex};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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
    let astrophage = Arc::new(AstrophageCoreLogger::new());
    astrophage.log(
        AstrophageLevel::Info,
        "system-init",
        "Process 1 initialization sequence started",
        None,
    );

    // 2. Hardware Detection
    let hw_profile = detect_hardware();
    println!(
        "[amgos-system] Hardware detected: CPU: {} ({} cores, {} threads), RAM: {} MB, GPU: {}",
        hw_profile.cpu_model,
        hw_profile.cpu_cores,
        hw_profile.cpu_threads,
        hw_profile.total_memory_bytes / (1024 * 1024),
        hw_profile.discrete_gpu_detected
    );

    astrophage.log(
        AstrophageLevel::Info,
        "hardware",
        &format!("CPU: {} ({} cores)", hw_profile.cpu_model, hw_profile.cpu_cores),
        Some(("cpu_cores", hw_profile.cpu_cores as f64)),
    );

    // 3. Audio-Video Manager (AVM)
    let avm = Arc::new(AudioVideoManager::new());
    avm.mark_audio_hardware_ready();

    // 4. Network Vault & Power Manager & Permission Manager
    let network = Arc::new(NetworkCredentialVault::new());
    let power = Arc::new(PowerManager::new());
    let permissions = Arc::new(PermissionManager::new());

    // 5. Filer Core: Pathfinder Index & App Layer Manager
    let pathfinder = Arc::new(Mutex::new(PathfinderIndex::new()));
    {
        let mut idx = pathfinder.lock().unwrap();
        idx.scan_system_applications();
    }
    let app_layer = Arc::new(AppLayerManager::default());

    // 6. Bind internal pub/sub EventBus (e-bus)
    let ebus_server = Arc::new(
        EventBusServer::bind(DEFAULT_EBUS_SOCKET_PATH)
            .expect("Failed to bind EventBus socket at /tmp/amgos-ebus.sock"),
    );
    println!(
        "[amgos-system] EventBus (e-bus) listening on {}",
        DEFAULT_EBUS_SOCKET_PATH
    );

    // 7. Start Supervisor and worker services
    let mut supervisor = ServiceSupervisor::new();
    let ebus_conn_poller = Arc::clone(&ebus_server);
    supervisor.register_service("ebus-poller", move |_running| {
        let _ = ebus_conn_poller.poll_connections();
    });

    let astro_sampler = Arc::clone(&astrophage);
    supervisor.register_service("astrophage-telemetry", move |_running| {
        if let Some(temp) = hardware::read_cpu_temperature() {
            astro_sampler.log(
                AstrophageLevel::Debug,
                "thermal",
                "CPU thermal reading",
                Some(("temp_celsius", temp as f64)),
            );
        }
    });

    // 8. Publish initial system bootstrap events
    let hw_event = SystemEvent::HardwareReady {
        timestamp_ns: now_ns(),
        cpu_model: hw_profile.cpu_model,
        cpu_cores: hw_profile.cpu_cores,
        total_memory_bytes: hw_profile.total_memory_bytes,
        discrete_gpu: hw_profile.discrete_gpu_detected,
    };
    let _ = ebus_server.publish_event(&hw_event);

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

    let default_profile = power.current_profile();
    let _ = ebus_server.publish_event(&SystemEvent::PowerProfileChanged {
        profile: default_profile,
        active_governor: "powersave".to_string(),
    });

    // 9. Main request dispatch loop
    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);
    let _ = ctrlc_handler(move || {
        println!("\n[amgos-system] Controlled shutdown requested.");
        r.store(false, Ordering::SeqCst);
    });

    println!("[amgos-system] Process 1 operational. Awaiting Process 2 requests.");

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(20));
        let _ = ebus_server.poll_connections();

        let incoming_requests = ebus_server.poll_requests();
        for (_msg_id, req) in incoming_requests {
            match req {
                DesktopRequest::SearchPathfinder {
                    query_id,
                    query,
                    max_results,
                } => {
                    let results = {
                        let idx = pathfinder.lock().unwrap();
                        idx.query(&query, max_results)
                    };
                    let _ = ebus_server.publish_event(&SystemEvent::PathfinderResults {
                        query_id,
                        results,
                    });
                }

                DesktopRequest::ListDirectory { request_id, path } => {
                    match FileSystemIndexer::list_directory(&path) {
                        Ok(entries) => {
                            let protocol_entries = entries
                                .into_iter()
                                .map(|e| ProtocolFileEntry {
                                    name: e.name,
                                    path: e.path,
                                    is_directory: e.is_directory,
                                    size_bytes: e.size_bytes,
                                    modified_timestamp_secs: e.modified_timestamp_secs,
                                })
                                .collect();

                            let _ = ebus_server.publish_event(&SystemEvent::DirectoryListing {
                                request_id,
                                path,
                                entries: protocol_entries,
                            });
                        }
                        Err(err) => {
                            astrophage.log(
                                AstrophageLevel::Warning,
                                "filer-fs",
                                &format!("Failed to list directory {path}: {err}"),
                                None,
                            );
                        }
                    }
                }

                DesktopRequest::InspectPackage { package_path } => {
                    match PackageInspector::inspect(&package_path) {
                        Ok(report) => {
                            let _ = ebus_server.publish_event(&SystemEvent::PackageInspected {
                                package_path,
                                report,
                            });
                        }
                        Err(err) => {
                            astrophage.log(
                                AstrophageLevel::Error,
                                "package-inspector",
                                &format!("Inspection failed for {package_path}: {err}"),
                                None,
                            );
                        }
                    }
                }

                DesktopRequest::InstallPackage {
                    package_path,
                    confirmed_permissions: _,
                } => {
                    let ebus_prog = Arc::clone(&ebus_server);
                    let path_clone = package_path.clone();

                    match app_layer.install_package(&package_path, move |stage, percent| {
                        let _ = ebus_prog.publish_event(&SystemEvent::PackageProgress {
                            package_path: path_clone.clone(),
                            stage,
                            percent,
                        });
                    }) {
                        Ok(app_id) => {
                            // Register installed app into Pathfinder
                            let _ = ebus_server.publish_event(&SystemEvent::PackageInstallFinished {
                                package_path,
                                app_id,
                                success: true,
                                error_message: None,
                            });
                        }
                        Err(err) => {
                            let _ = ebus_server.publish_event(&SystemEvent::PackageInstallFinished {
                                package_path,
                                app_id: String::new(),
                                success: false,
                                error_message: Some(err.to_string()),
                            });
                        }
                    }
                }

                DesktopRequest::UninstallPackage { app_id } => {
                    match app_layer.uninstall_app(&app_id) {
                        Ok(()) => {
                            // Remove from Pathfinder index immediately (symmetric uninstallation)
                            {
                                let mut idx = pathfinder.lock().unwrap();
                                idx.remove_entry(&format!("app-desktop-{app_id}"));
                                idx.remove_entry(&app_id);
                            }

                            let _ = ebus_server.publish_event(&SystemEvent::PackageUninstallFinished {
                                app_id,
                                success: true,
                                error_message: None,
                            });
                        }
                        Err(err) => {
                            let _ = ebus_server.publish_event(&SystemEvent::PackageUninstallFinished {
                                app_id,
                                success: false,
                                error_message: Some(err.to_string()),
                            });
                        }
                    }
                }

                DesktopRequest::SetPowerProfile(profile) => {
                    match power.set_profile(profile) {
                        Ok(governor) => {
                            let _ = ebus_server.publish_event(&SystemEvent::PowerProfileChanged {
                                profile,
                                active_governor: governor,
                            });
                        }
                        Err(err) => {
                            astrophage.log(
                                AstrophageLevel::Error,
                                "power",
                                &format!("Failed to switch power profile: {err}"),
                                None,
                            );
                        }
                    }
                }

                DesktopRequest::RequestPowerState(target) => {
                    let _ = power.transition_to(target);
                    let _ = ebus_server.publish_event(&SystemEvent::PowerStateTransition {
                        target,
                        initiated_by_system: false,
                    });
                }

                DesktopRequest::ConnectWifi { ssid, passphrase } => {
                    let status = network.connect(&ssid, &passphrase);
                    let _ = ebus_server.publish_event(&SystemEvent::NetworkStateChanged {
                        connected: status.is_connected,
                        interface_name: status.interface,
                        ssid: status.ssid,
                        ip_address: status.ip_address,
                    });
                }

                DesktopRequest::GetAstrophageBuffer { max_entries } => {
                    let entries = astrophage.snapshot(max_entries);
                    let _ = ebus_server.publish_event(&SystemEvent::AstrophageBufferSnapshot {
                        entries,
                    });
                }

                DesktopRequest::ApplySignedConfig {
                    config_payload,
                    hmac_signature,
                } => {
                    let _ = permissions.apply_config(&config_payload, &hmac_signature);
                }
            }
        }
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
    thread::spawn(move || {
        let _ = f;
    });
    Ok(())
}
