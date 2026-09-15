//! Network Credential Vault and Connection Manager
//!
//! Exclusively owned by Process 1. Network credentials NEVER leave this module.
//! Process 2 receives only sanitized NetworkStatus events via e-bus (no SSID password).
//!
//! Connection backend: NetworkManager D-Bus (org.freedesktop.NetworkManager).
//! Fallback: direct wpa_supplicant / wpa_cli if NM is unavailable.
//!
//! WiFi scanning: reads /proc/net/wireless + system nmcli for AP list.
//! Credential storage: in-process memory only. On first boot wizard completion,
//! NM stores the credential in its own encrypted keyfile in /etc/NetworkManager/system-connections/
//! (owned by root, not readable by any user process).
//!
//! The credential HashMap here acts as a Process 1 runtime cache for quick reconnect
//! decisions, not as persistent storage. NM is the persistent owner.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

/// A snapshot of current network state (safe to send over e-bus to Process 2)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkStatus {
    pub interface: String,
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
    pub is_connected: bool,
    pub signal_strength_pct: Option<u8>,
}

/// A discovered WiFi access point (safe to send to Process 2 for display — no credentials)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    pub ssid: String,
    pub signal_strength_pct: u8,
    pub is_secured: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkCredentialVault {
    // In-memory runtime credential cache. Never serialized to disk by this module.
    credentials: Arc<Mutex<HashMap<String, String>>>,
    active_connection: Arc<Mutex<Option<NetworkStatus>>>,
}

impl NetworkCredentialVault {
    pub fn new() -> Self {
        let (primary_iface, is_up) = detect_primary_interface();
        let ip = detect_local_ip_address();

        let initial_status = NetworkStatus {
            interface: primary_iface,
            ssid: detect_active_ssid(),
            ip_address: ip,
            is_connected: is_up,
            signal_strength_pct: None,
        };

        Self {
            credentials: Arc::new(Mutex::new(HashMap::new())),
            active_connection: Arc::new(Mutex::new(Some(initial_status))),
        }
    }

    /// Store a WiFi credential in the Process 1 runtime cache.
    /// Separately, persist via NetworkManager so the credential survives reboot.
    pub fn store_credential(&self, ssid: &str, passphrase: &str) {
        if let Ok(mut creds) = self.credentials.lock() {
            creds.insert(ssid.to_string(), passphrase.to_string());
        }
        // Persist to NetworkManager via nmcli (NM owns the on-disk keyfile)
        persist_to_networkmanager(ssid, passphrase);
    }

    /// Connect to a WiFi network using NetworkManager as the connection backend.
    /// Credentials are accepted from Process 2 via e-bus, stored internally,
    /// and passed to NM — Process 2 never receives them back.
    pub fn connect(&self, ssid: &str, passphrase: &str) -> NetworkStatus {
        self.store_credential(ssid, passphrase);

        // Ask NetworkManager to activate the connection
        let connected = nm_connect_wifi(ssid, passphrase);

        let (primary_iface, _) = detect_primary_interface();
        let ip = if connected {
            // Give NM a moment for DHCP, then read the address
            std::thread::sleep(std::time::Duration::from_millis(500));
            detect_local_ip_address()
        } else {
            None
        };

        let status = NetworkStatus {
            interface: primary_iface,
            ssid: if connected { Some(ssid.to_string()) } else { None },
            ip_address: ip,
            is_connected: connected,
            signal_strength_pct: None,
        };

        if let Ok(mut active) = self.active_connection.lock() {
            *active = Some(status.clone());
        }

        status
    }

    /// Disconnect from the current active WiFi connection
    pub fn disconnect(&self) -> NetworkStatus {
        nm_disconnect_wifi();

        let (primary_iface, _) = detect_primary_interface();
        let status = NetworkStatus {
            interface: primary_iface,
            ssid: None,
            ip_address: None,
            is_connected: false,
            signal_strength_pct: None,
        };

        if let Ok(mut active) = self.active_connection.lock() {
            *active = Some(status.clone());
        }

        status
    }

    /// Returns the current network status without credentials
    pub fn current_status(&self) -> NetworkStatus {
        // Always read live from system state to avoid stale cache
        let (primary_iface, is_up) = detect_primary_interface();
        let ip = detect_local_ip_address();
        let ssid = detect_active_ssid();

        let status = NetworkStatus {
            interface: primary_iface,
            ssid,
            ip_address: ip,
            is_connected: is_up,
            signal_strength_pct: None,
        };

        // Update cache
        if let Ok(mut active) = self.active_connection.lock() {
            *active = Some(status.clone());
        }

        status
    }

    /// Scan for available WiFi access points (no credentials returned)
    pub fn scan_access_points(&self) -> Vec<AccessPoint> {
        scan_wifi_aps()
    }

    /// List all discovered physical network interfaces
    pub fn list_interfaces(&self) -> Vec<String> {
        let mut ifaces = Vec::new();
        let net_dir = Path::new("/sys/class/net");
        if let Ok(entries) = fs::read_dir(net_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "lo"
                    && !name.starts_with("virbr")
                    && !name.starts_with("docker")
                    && !name.starts_with("veth")
                {
                    ifaces.push(name);
                }
            }
        }
        ifaces.sort();
        ifaces
    }
}

// ---------------------------------------------------------------------------
// NetworkManager integration (nmcli-based, no shell scripts — nmcli is the
// binary interface to the NetworkManager D-Bus API)
// ---------------------------------------------------------------------------

/// Connect via NetworkManager. Returns true if NM reports active connection.
fn nm_connect_wifi(ssid: &str, passphrase: &str) -> bool {
    // nmcli device wifi connect <ssid> password <passphrase>
    let output = Command::new("nmcli")
        .args(["device", "wifi", "connect", ssid, "password", passphrase])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => {
            // NM not available — fall back to wpa_cli
            wpa_cli_connect(ssid, passphrase)
        }
    }
}

/// Disconnect via NetworkManager
fn nm_disconnect_wifi() {
    let (iface, _) = detect_primary_interface();
    let _ = Command::new("nmcli")
        .args(["device", "disconnect", &iface])
        .status();
}

/// Persist WiFi credentials to NetworkManager's keyfile store.
/// This creates a persistent connection profile owned by root.
fn persist_to_networkmanager(ssid: &str, passphrase: &str) {
    // nmcli connection add type wifi ssid <ssid> wifi-sec.key-mgmt wpa-psk
    //         wifi-sec.psk <passphrase> -- only if connection does not already exist
    let existing = Command::new("nmcli")
        .args(["connection", "show", ssid])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !existing {
        let _ = Command::new("nmcli")
            .args([
                "connection",
                "add",
                "type",
                "wifi",
                "ssid",
                ssid,
                "wifi-sec.key-mgmt",
                "wpa-psk",
                "wifi-sec.psk",
                passphrase,
                "connection.autoconnect",
                "yes",
            ])
            .status();
    }
}

/// wpa_cli fallback for systems without NetworkManager
fn wpa_cli_connect(ssid: &str, passphrase: &str) -> bool {
    let (iface, _) = detect_primary_interface();

    // Add network
    let network_id_output = Command::new("wpa_cli")
        .args(["-i", &iface, "add_network"])
        .output();

    let network_id = match network_id_output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => return false,
    };

    let nid = network_id.trim();

    // Configure SSID
    let _ = Command::new("wpa_cli")
        .args(["-i", &iface, "set_network", nid, "ssid", &format!("\"{ssid}\"")])
        .status();

    // Configure passphrase
    let _ = Command::new("wpa_cli")
        .args(["-i", &iface, "set_network", nid, "psk", &format!("\"{passphrase}\"")])
        .status();

    // Enable and select
    let _ = Command::new("wpa_cli")
        .args(["-i", &iface, "enable_network", nid])
        .status();

    let status = Command::new("wpa_cli")
        .args(["-i", &iface, "select_network", nid])
        .status();

    status.map(|s| s.success()).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Access point scanning
// ---------------------------------------------------------------------------

/// Scan WiFi APs using nmcli. Falls back to /proc/net/wireless for basic info.
fn scan_wifi_aps() -> Vec<AccessPoint> {
    // Trigger a rescan
    let _ = Command::new("nmcli")
        .args(["device", "wifi", "rescan"])
        .status();

    // nmcli -t -f SSID,SIGNAL,SECURITY device wifi list
    let output = Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            parse_nmcli_wifi_list(&text)
        }
        _ => {
            // nmcli unavailable — minimal fallback from /proc/net/wireless
            proc_net_wireless_aps()
        }
    }
}

/// Parse `nmcli -t -f SSID,SIGNAL,SECURITY device wifi list` output
fn parse_nmcli_wifi_list(text: &str) -> Vec<AccessPoint> {
    let mut aps = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            let ssid = parts[0].trim().to_string();
            if ssid.is_empty() {
                continue;
            }
            let signal: u8 = parts[1].trim().parse().unwrap_or(0);
            let security = parts[2].trim();
            aps.push(AccessPoint {
                ssid,
                signal_strength_pct: signal,
                is_secured: !security.is_empty() && security != "--",
            });
        }
    }
    // Deduplicate by SSID, keeping the strongest signal
    aps.sort_by(|a, b| b.signal_strength_pct.cmp(&a.signal_strength_pct));
    aps.dedup_by(|a, b| a.ssid == b.ssid);
    aps
}

/// Minimal AP list from /proc/net/wireless (no SSID names, only interface stats)
fn proc_net_wireless_aps() -> Vec<AccessPoint> {
    // /proc/net/wireless doesn't provide AP SSID list — return empty
    Vec::new()
}

// ---------------------------------------------------------------------------
// Interface and IP detection
// ---------------------------------------------------------------------------

fn detect_primary_interface() -> (String, bool) {
    let net_dir = Path::new("/sys/class/net");
    let mut best: Option<(String, bool)> = None;

    if let Ok(entries) = fs::read_dir(net_dir) {
        let mut ifaces: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| {
                // Prefer wireless (wlo*, wlp*, wlan*) then ethernet (eno*, enp*, eth*)
                n.starts_with('w') || n.starts_with('e')
            })
            .collect();

        // Prefer interfaces that are "up" and operational
        ifaces.sort();
        for name in ifaces {
            let operstate = fs::read_to_string(format!("/sys/class/net/{name}/operstate"))
                .unwrap_or_default()
                .trim()
                .to_string();
            let is_up = operstate == "up";
            if is_up || best.is_none() {
                best = Some((name, is_up));
                if is_up {
                    break; // Found an active interface
                }
            }
        }
    }

    best.unwrap_or_else(|| ("wlo1".to_string(), false))
}

fn detect_active_ssid() -> Option<String> {
    // Read from NetworkManager via nmcli (most reliable source)
    let output = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,SSID", "device", "wifi"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() == 2 && parts[0].trim() == "yes" {
            let ssid = parts[1].trim().to_string();
            if !ssid.is_empty() {
                return Some(ssid);
            }
        }
    }

    // Fallback: read from /proc/net/wireless (no SSID name, just interface presence)
    None
}

fn detect_local_ip_address() -> Option<String> {
    // Primary: read from /proc/net/fib_trie (host LOCAL /32 entries)
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
                    && !ip_str.starts_with("169.254.")
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

    // Fallback: nmcli -g IP4.ADDRESS device show <iface>
    let (iface, _) = detect_primary_interface();
    let output = Command::new("nmcli")
        .args(["-g", "IP4.ADDRESS", "device", "show", &iface])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        // IP4.ADDRESS output is like "192.168.1.100/24"
        if !trimmed.is_empty() && trimmed.contains('/') {
            return Some(trimmed.split('/').next()?.to_string());
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
    fn test_vault_interface_detection() {
        let vault = NetworkCredentialVault::new();
        let status = vault.current_status();
        assert!(!status.interface.is_empty(), "Must detect at least one interface");
    }

    #[test]
    fn test_vault_list_interfaces() {
        let vault = NetworkCredentialVault::new();
        let ifaces = vault.list_interfaces();
        assert!(!ifaces.is_empty(), "Should detect at least loopback or physical interfaces");
    }

    #[test]
    fn test_vault_credential_store_isolation() {
        let vault = NetworkCredentialVault::new();
        vault.store_credential("TestNet-AMGOS", "SecurePassword123!");
        // Verify credential is in runtime cache but never returned to caller
        let creds = vault.credentials.lock().unwrap();
        assert!(creds.contains_key("TestNet-AMGOS"));
        // No public accessor for the passphrase — by design
    }

    #[test]
    fn test_nmcli_wifi_list_parser() {
        let sample = "HomeNet:85:WPA2\nGuest:42:--\n:0:WPA3\n";
        let aps = parse_nmcli_wifi_list(sample);
        assert_eq!(aps.len(), 2); // Empty SSID row filtered
        assert_eq!(aps[0].ssid, "HomeNet");
        assert_eq!(aps[0].signal_strength_pct, 85);
        assert!(aps[0].is_secured);
        assert_eq!(aps[1].ssid, "Guest");
        assert!(!aps[1].is_secured);
    }

    #[test]
    fn test_status_no_crash_when_disconnected() {
        let vault = NetworkCredentialVault::new();
        let status = vault.current_status();
        // Should not panic even on a machine with no active wifi
        let _ = status.is_connected;
        let _ = status.ip_address;
    }
}
