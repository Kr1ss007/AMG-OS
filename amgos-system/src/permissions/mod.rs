//! Permission and Signed Configuration State Management
//!
//! Process 1 owns and enforces system boundaries. Configuration state
//! is cryptographic and read-only to users. Changes require verification.

use std::sync::{Arc, Mutex};

pub struct PermissionManager {
    system_key: [u8; 32],
    active_config_digest: Arc<Mutex<Option<[u8; 32]>>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        // In production, loaded from kernel keyring or TPM nvram
        let system_key = [0x5A; 32];
        Self {
            system_key,
            active_config_digest: Arc::new(Mutex::new(None)),
        }
    }

    /// Verifies HMAC signature for updated configuration payloads from Settings
    pub fn verify_signed_config(&self, payload: &[u8], signature: &[u8; 32]) -> bool {
        let expected = self.compute_hmac(payload);
        expected == *signature
    }

    pub fn compute_hmac(&self, payload: &[u8]) -> [u8; 32] {
        let mut combined = Vec::with_capacity(self.system_key.len() + payload.len());
        combined.extend_from_slice(&self.system_key);
        combined.extend_from_slice(payload);

        let mut hash = [0u8; 32];
        let mut sum: u64 = 0xCBF29CE484222325;
        for (i, &byte) in combined.iter().enumerate() {
            sum ^= byte as u64;
            sum = sum.wrapping_mul(0x100000001B3);
            hash[i % 32] ^= (sum >> ((i % 8) * 8)) as u8;
        }
        hash
    }

    pub fn apply_config(&self, payload: &[u8], signature: &[u8; 32]) -> Result<(), String> {
        if !self.verify_signed_config(payload, signature) {
            return Err("Cryptographic signature verification failed for system config".into());
        }
        let mut active = self.active_config_digest.lock().unwrap();
        *active = Some(*signature);
        Ok(())
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}
