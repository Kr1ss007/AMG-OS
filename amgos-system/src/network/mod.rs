//! Network Credential Vault and Connection Manager
//!
//! Owns all network credentials and interface states.
//! Strictly kept inside Process 1. Never exposed to Process 2.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct NetworkCredentialVault {
    // Stored in Process 1 memory, isolated from desktop session
    credentials: Arc<Mutex<HashMap<String, String>>>,
    active_connection: Arc<Mutex<Option<NetworkStatus>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkStatus {
    pub interface: String,
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
    pub is_connected: bool,
}

impl NetworkCredentialVault {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(Mutex::new(HashMap::new())),
            active_connection: Arc::new(Mutex::new(Some(NetworkStatus {
                interface: "wlan0".to_string(),
                ssid: None,
                ip_address: None,
                is_connected: false,
            }))),
        }
    }

    /// Stores a network credential in Process 1 vault
    pub fn store_credential(&self, ssid: &str, passphrase: &str) {
        if let Ok(mut creds) = self.credentials.lock() {
            creds.insert(ssid.to_string(), passphrase.to_string());
        }
    }

    /// Attempts connection using stored or provided credentials
    pub fn connect(&self, ssid: &str, passphrase: &str) -> NetworkStatus {
        self.store_credential(ssid, passphrase);
        let status = NetworkStatus {
            interface: "wlan0".to_string(),
            ssid: Some(ssid.to_string()),
            ip_address: Some("192.168.1.105".to_string()),
            is_connected: true,
        };

        if let Ok(mut active) = self.active_connection.lock() {
            *active = Some(status.clone());
        }

        status
    }

    pub fn current_status(&self) -> NetworkStatus {
        if let Ok(active) = self.active_connection.lock() {
            if let Some(ref status) = *active {
                return status.clone();
            }
        }
        NetworkStatus {
            interface: "eth0".to_string(),
            ssid: None,
            ip_address: None,
            is_connected: false,
        }
    }
}

impl Default for NetworkCredentialVault {
    fn default() -> Self {
        Self::new()
    }
}
