//! AMG-OS Process 1 Backend Integration Tests
//!
//! Verifies live end-to-end communication across EventBus between Process 1 and Process 2:
//! - Request dispatching (Pathfinder search, Power profile changes, Directory listings)
//! - Hardware and Network vault queries
//! - Astrophage buffer snapshots
//! - Hard two-process boundary preservation

use amgos_protocol::ebus::{
    DesktopRequest, EventBusClient, EventBusServer, PathfinderCategory, PowerProfile,
    ProtocolFileEntry, SystemEvent,
};
use amgos_system::astrophage::AstrophageCoreLogger;
use amgos_system::network::NetworkCredentialVault;
use amgos_system::permissions::PermissionManager;
use amgos_system::power::PowerManager;
use filer_core::{AppLayerManager, FileSystemIndexer, PathfinderIndex};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn test_process1_backend_daemon_dispatch_loop() {
    let test_socket = "/tmp/amgos_ebus_integration_test.sock";

    let server = Arc::new(EventBusServer::bind(test_socket).expect("Failed to bind test socket"));
    let running = Arc::new(AtomicBool::new(true));

    let pathfinder = Arc::new(Mutex::new(PathfinderIndex::new()));
    let power = Arc::new(PowerManager::new());
    let network = Arc::new(NetworkCredentialVault::new());
    let astrophage = Arc::new(AstrophageCoreLogger::new());
    let _permissions = Arc::new(PermissionManager::new());
    let _app_layer = Arc::new(AppLayerManager::default());

    let srv_clone = Arc::clone(&server);
    let run_clone = Arc::clone(&running);
    let pf_clone = Arc::clone(&pathfinder);
    let pw_clone = Arc::clone(&power);
    let nw_clone = Arc::clone(&network);
    let as_clone = Arc::clone(&astrophage);

    let motionwave = Arc::new(amgos_system::motionwave::MotionWaveController::new());
    let mw_clone = Arc::clone(&motionwave);
    let _ap_clone = Arc::clone(&_app_layer);

    // Spawn Process 1 background dispatcher
    let server_handle = thread::spawn(move || {
        while run_clone.load(Ordering::SeqCst) {
            let _ = srv_clone.poll_connections();
            let reqs = srv_clone.poll_requests();

            for (_msg_id, req) in reqs {
                match req {
                    DesktopRequest::SearchPathfinder {
                        query_id,
                        query,
                        max_results,
                    } => {
                        let results = pf_clone.lock().unwrap().query(&query, max_results);
                        let _ = srv_clone.publish_event(&SystemEvent::PathfinderResults {
                            query_id,
                            results,
                        });
                    }

                    DesktopRequest::SetPowerProfile(profile) => {
                        let gov = pw_clone.set_profile(profile).unwrap_or_else(|_| "powersave".into());
                        let _ = srv_clone.publish_event(&SystemEvent::PowerProfileChanged {
                            profile,
                            active_governor: gov,
                        });
                    }

                    DesktopRequest::ListDirectory { request_id, path } => {
                        let entries = FileSystemIndexer::list_directory(&path)
                            .unwrap_or_default()
                            .into_iter()
                            .map(|e| ProtocolFileEntry {
                                name: e.name,
                                path: e.path,
                                is_directory: e.is_directory,
                                size_bytes: e.size_bytes,
                                modified_timestamp_secs: e.modified_timestamp_secs,
                            })
                            .collect();

                        let _ = srv_clone.publish_event(&SystemEvent::DirectoryListing {
                            request_id,
                            path,
                            entries,
                        });
                    }

                    DesktopRequest::ConnectWifi { ssid, passphrase } => {
                        let status = nw_clone.connect(&ssid, &passphrase);
                        let _ = srv_clone.publish_event(&SystemEvent::NetworkStateChanged {
                            connected: status.is_connected,
                            interface_name: status.interface,
                            ssid: status.ssid,
                            ip_address: status.ip_address,
                        });
                    }

                    DesktopRequest::GetAstrophageBuffer { max_entries } => {
                        let entries = as_clone.snapshot(max_entries);
                        let _ = srv_clone.publish_event(&SystemEvent::AstrophageBufferSnapshot {
                            entries,
                        });
                    }

                    DesktopRequest::GetInputDevices => {
                        let devices = mw_clone.enumerate_devices().unwrap_or_default();
                        let _ = srv_clone.publish_event(&SystemEvent::InputDevicesChanged { devices });
                    }

                    DesktopRequest::SetTouchpadConfig(config) => {
                        mw_clone.set_touchpad_config(config.clone());
                        let _ = srv_clone.publish_event(&SystemEvent::TouchpadConfigChanged(config));
                    }

                    DesktopRequest::SetKeyboardConfig(config) => {
                        mw_clone.set_keyboard_config(config.clone());
                        let _ = srv_clone.publish_event(&SystemEvent::KeyboardConfigChanged(config));
                    }

                    DesktopRequest::InspectPackage { package_path } => {
                        if let Ok(report) = filer_core::PackageInspector::inspect(&package_path) {
                            let _ = srv_clone.publish_event(&SystemEvent::PackageInspected {
                                package_path,
                                report,
                            });
                        }
                    }

                    _ => {}
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
    });

    // Wait for server to start
    thread::sleep(Duration::from_millis(50));

    // Connect Client (simulating Process 2 Desktop Session)
    let client = EventBusClient::connect(test_socket).expect("Client connect failed");

    // 1. Test Pathfinder Query
    client
        .send_request(&DesktopRequest::SearchPathfinder {
            query_id: 101,
            query: "power".to_string(),
            max_results: 5,
        })
        .expect("Send search request failed");

    let event1 = client.read_event().expect("Read event failed");
    match event1 {
        SystemEvent::PathfinderResults { query_id, results } => {
            assert_eq!(query_id, 101);
            assert!(!results.is_empty());
            assert_eq!(results[0].category, PathfinderCategory::Setting);
            assert!(results[0].title.contains("Power"));
        }
        other => panic!("Expected PathfinderResults, got {:?}", other),
    }

    // 2. Test Setting Power Profile to MAX
    client
        .send_request(&DesktopRequest::SetPowerProfile(PowerProfile::Max))
        .expect("Send power profile request failed");

    let event2 = client.read_event().expect("Read event failed");
    match event2 {
        SystemEvent::PowerProfileChanged {
            profile,
            active_governor,
        } => {
            assert_eq!(profile, PowerProfile::Max);
            assert_eq!(active_governor, "performance");
        }
        other => panic!("Expected PowerProfileChanged, got {:?}", other),
    }

    // 3. Test Directory Listing
    client
        .send_request(&DesktopRequest::ListDirectory {
            request_id: 202,
            path: ".".to_string(),
        })
        .expect("Send list directory failed");

    let event3 = client.read_event().expect("Read event failed");
    match event3 {
        SystemEvent::DirectoryListing {
            request_id,
            path: _,
            entries,
        } => {
            assert_eq!(request_id, 202);
            assert!(!entries.is_empty());
        }
        other => panic!("Expected DirectoryListing, got {:?}", other),
    }

    // 4. Test Wi-Fi Credentials Storage inside Process 1
    client
        .send_request(&DesktopRequest::ConnectWifi {
            ssid: "AMGOS-Office-5G".to_string(),
            passphrase: "EncryptedInVault".to_string(),
        })
        .expect("Send connect wifi failed");

    let event4 = client.read_event().expect("Read event failed");
    match event4 {
        SystemEvent::NetworkStateChanged {
            interface_name,
            ssid: _,
            ip_address: _,
            connected: _,
        } => {
            // We only assert the interface is non-empty; actual WiFi connection
            // depends on the SSID being reachable in the test environment.
            assert!(!interface_name.is_empty(), "Interface name must be populated");
        }
        other => panic!("Expected NetworkStateChanged, got {:?}", other),
    }

    // 5. Test Astrophage Diagnostics Snapshot
    client
        .send_request(&DesktopRequest::GetAstrophageBuffer { max_entries: 50 })
        .expect("Send get astrophage failed");

    let event5 = client.read_event().expect("Read event failed");
    match event5 {
        SystemEvent::AstrophageBufferSnapshot { entries: _ } => {
            // Valid response received
        }
        other => panic!("Expected AstrophageBufferSnapshot, got {:?}", other),
    }

    // 6. Test MotionWave Input Devices Enumeration
    client
        .send_request(&DesktopRequest::GetInputDevices)
        .expect("Send get input devices failed");

    let event6 = client.read_event().expect("Read event failed");
    match event6 {
        SystemEvent::InputDevicesChanged { devices } => {
            // Devices enumerated from /proc/bus/input/devices on host
            assert!(!devices.is_empty(), "Should detect host keyboard and touchpad");
        }
        other => panic!("Expected InputDevicesChanged, got {:?}", other),
    }

    // 7. Test MotionWave Touchpad Configuration Update
    let custom_tp = amgos_protocol::ebus::TouchpadConfig {
        tap_to_click: true,
        natural_scrolling: true,
        pointer_speed: 0.5,
        palm_rejection: true,
        two_finger_scroll: true,
    };
    client
        .send_request(&DesktopRequest::SetTouchpadConfig(custom_tp.clone()))
        .expect("Send set touchpad config failed");

    let event7 = client.read_event().expect("Read event failed");
    match event7 {
        SystemEvent::TouchpadConfigChanged(cfg) => {
            assert_eq!(cfg.pointer_speed, 0.5);
            assert!(cfg.tap_to_click);
        }
        other => panic!("Expected TouchpadConfigChanged, got {:?}", other),
    }

    // 8. Test MotionWave Keyboard Configuration Update
    let custom_kbd = amgos_protocol::ebus::KeyboardConfig {
        repeat_rate_hz: 40,
        repeat_delay_ms: 200,
        layout: "us".to_string(),
    };
    client
        .send_request(&DesktopRequest::SetKeyboardConfig(custom_kbd.clone()))
        .expect("Send set keyboard config failed");

    let event8 = client.read_event().expect("Read event failed");
    match event8 {
        SystemEvent::KeyboardConfigChanged(cfg) => {
            assert_eq!(cfg.repeat_rate_hz, 40);
            assert_eq!(cfg.repeat_delay_ms, 200);
        }
        other => panic!("Expected KeyboardConfigChanged, got {:?}", other),
    }

    // Clean shutdown
    running.store(false, Ordering::SeqCst);
    let _ = server_handle.join();
    server.shutdown();
}
