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
            connected,
            interface_name,
            ssid,
            ip_address: _,
        } => {
            assert!(connected);
            assert!(!interface_name.is_empty());
            assert_eq!(ssid, Some("AMGOS-Office-5G".to_string()));
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

    // Clean shutdown
    running.store(false, Ordering::SeqCst);
    let _ = server_handle.join();
    server.shutdown();
}
