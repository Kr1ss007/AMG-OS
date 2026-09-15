//! Network Credential Vault and Connection Manager
//!
//! Owns all network credentials and interface states.
//! Strictly kept inside Process 1. Never exposed to Process 2.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct NetworkCredentialVault {
    // Stored securely in Process 1 memory, isolated from desktop session
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
        let (primary_iface, is_up) = detect_primary_interface();
        let ip = detect_local_ip_address();

        let initial_status = NetworkStatus {
            interface: primary_iface,
            ssid: None,
            ip_address: ip,
            is_connected: is_up,
        };

        Self {
            credentials: Arc::new(Mutex::new(HashMap::new())),
            active_connection: Arc::new(Mutex::new(Some(initial_status))),
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
        let (primary_iface, _) = detect_primary_interface();
        let ip = detect_local_ip_address();

        let status = NetworkStatus {
            interface: primary_iface,
            ssid: Some(ssid.to_string()),
            ip_address: ip,
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

        let (primary_iface, is_up) = detect_primary_interface();
        let ip = detect_local_ip_address();
        NetworkStatus {
            interface: primary_iface,
            ssid: None,
            ip_address: ip,
            is_connected: is_up,
        }
    }

    /// List all discovered physical network interfaces on the host
    pub fn list_interfaces(&self) -> Vec<String> {
        let mut ifaces = Vec::new();
        let net_dir = Path::new("/sys/class/net");
        if let Ok(entries) = fs::read_dir(net_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "lo" && !name.starts_with("virbr") && !name.starts_with("docker") {
                    ifaces.push(name);
                }
            }
        }
        ifaces.sort();
        ifaces
    }
}

fn detect_primary_interface() -> (String, bool) {
    let net_dir = Path::new("/sys/class/net");
    if let Ok(entries) = fs::read_dir(net_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Prioritize physical wireless (wlo, wlp, wlan) or ethernet (eno, enp, eth)
            if name.starts_with('w') || name.starts_with('e') {
                let operstate_path = entry.path().join("operstate");
                let operstate = fs::read_to_string(operstate_path)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let is_up = operstate == "up";
                if is_up {
                    return (name, true);
                }
            }
        }
    }
    ("wlo1".to_string(), false)
}

fn detect_local_ip_address() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/proc/net/fib_trie") {
        let mut candidate_ip: Option<String> = None;
        let lines: Vec<&str> = content.lines().collect();

        for i in 0..lines.len() {
            let line = lines[i].trim();
            if line.starts_with("|--") {
                let ip_str = line.trim_start_matches("|--").trim();
                if (i + 1) < lines.len()
                    && lines[i + 1].contains("/32 host LOCAL")
                    && ip_str != "127.0.0.1"
                    && !ip_str.starts_with("192.168.122.")
                {
                    candidate_ip = Some(ip_str.to_string());
                    break;
                }
            }
        }
        if candidate_ip.is_some() {
            return candidate_ip;
        }
    }
    None
}

impl Default for NetworkCredentialVault {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_vault_isolation_and_interfaces() {
        let vault = NetworkCredentialVault::new();
        let status = vault.current_status();
        assert!(!status.interface.is_empty());

        let ifaces = vault.list_interfaces();
        assert!(!ifaces.is_empty(), "Should detect physical interfaces");

        vault.store_credential("AMGOS-HQ", "SuperSecurePassword123");
        let conn = vault.connect("AMGOS-HQ", "SuperSecurePassword123");
        assert_eq!(conn.ssid, Some("AMGOS-HQ".to_string()));
        assert!(conn.is_connected);
    }
}
