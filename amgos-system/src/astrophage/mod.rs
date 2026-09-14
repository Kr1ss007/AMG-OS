//! Astrophage Continuous Rolling Event Buffer (Process 1 Core)
//!
//! Maintains an in-memory ring buffer (up to 10,000 events) capturing
//! hardware health, thermal events, crashes, and performance metrics.

use amgos_protocol::ebus::{AstrophageLevel, AstrophageRecord};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const RING_BUFFER_CAPACITY: usize = 10_000;

#[derive(Clone)]
pub struct AstrophageCoreLogger {
    buffer: Arc<Mutex<VecDeque<AstrophageRecord>>>,
}

impl AstrophageCoreLogger {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(RING_BUFFER_CAPACITY))),
        }
    }

    pub fn log(
        &self,
        level: AstrophageLevel,
        subsystem: &str,
        message: &str,
        metric: Option<(&str, f64)>,
    ) {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let record = AstrophageRecord {
            timestamp_ns: now_ns,
            level,
            subsystem: subsystem.to_string(),
            message: message.to_string(),
            metric_key: metric.map(|(k, _)| k.to_string()),
            metric_value: metric.map(|(_, v)| v),
        };

        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() >= RING_BUFFER_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(record);
        }
    }

    pub fn snapshot(&self, limit: usize) -> Vec<AstrophageRecord> {
        if let Ok(buf) = self.buffer.lock() {
            let start = if buf.len() > limit {
                buf.len() - limit
            } else {
                0
            };
            buf.iter().skip(start).cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for AstrophageCoreLogger {
    fn default() -> Self {
        Self::new()
    }
}
