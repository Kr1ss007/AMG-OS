//! astrophage-core: Process 1 Continuous Diagnostic Logger
//!
//! Maintains rolling circular telemetry buffer in Process 1.
//! Monitors system temperatures, throttle events, and error logs.

use amgos_protocol::ebus::{AstrophageLevel, AstrophageRecord};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_MAX_LOG_ENTRIES: usize = 10_000;

pub struct AstrophageBuffer {
    capacity: usize,
    entries: Arc<Mutex<VecDeque<AstrophageRecord>>>,
}

impl AstrophageBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
        }
    }

    pub fn record(&self, level: AstrophageLevel, subsystem: &str, message: &str) {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let record = AstrophageRecord {
            timestamp_ns,
            level,
            subsystem: subsystem.to_string(),
            message: message.to_string(),
            metric_key: None,
            metric_value: None,
        };

        let mut queue = self.entries.lock().unwrap();
        if queue.len() >= self.capacity {
            queue.pop_front();
        }
        queue.push_back(record);
    }

    pub fn get_recent(&self, limit: usize) -> Vec<AstrophageRecord> {
        let queue = self.entries.lock().unwrap();
        let start = if queue.len() > limit {
            queue.len() - limit
        } else {
            0
        };
        queue.iter().skip(start).cloned().collect()
    }
}

impl Default for AstrophageBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_LOG_ENTRIES)
    }
}
