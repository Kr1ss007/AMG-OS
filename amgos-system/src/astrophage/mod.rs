//! Astrophage Continuous Rolling Event Buffer (Process 1 Core)
//!
//! Maintains rolling 10,000-entry ring buffer in Process 1.
//! Samples live Linux kernel logs via /dev/kmsg, tracks Pressure Stall Information (PSI),
//! and records hardware health, thermal events, crashes, and performance metrics.

pub use astrophage_core::{
    AstrophageBuffer, DiagnosticReportBuilder, KmsgReader, NvmeStatReport, PressureMetrics,
    PsiMonitor, StorageHealthMonitor, SystemPsiReport, DEFAULT_MAX_LOG_ENTRIES,
};

use amgos_protocol::ebus::{AstrophageLevel, AstrophageRecord};
use std::sync::Arc;

pub const RING_BUFFER_CAPACITY: usize = DEFAULT_MAX_LOG_ENTRIES;

#[derive(Clone)]
pub struct AstrophageCoreLogger {
    inner: Arc<AstrophageBuffer>,
}

impl AstrophageCoreLogger {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AstrophageBuffer::new(RING_BUFFER_CAPACITY)),
        }
    }

    pub fn log(
        &self,
        level: AstrophageLevel,
        subsystem: &str,
        message: &str,
        metric: Option<(&str, f64)>,
    ) {
        if let Some((k, v)) = metric {
            self.inner.record_metric(level, subsystem, message, k, v);
        } else {
            self.inner.record(level, subsystem, message);
        }
    }

    pub fn snapshot(&self, limit: usize) -> Vec<AstrophageRecord> {
        self.inner.get_recent(limit)
    }

    pub fn poll_kmsg(&self, max_lines: usize) {
        let records = KmsgReader::read_available(max_lines);
        for r in records {
            self.inner.record(r.level, &r.subsystem, &r.message);
        }
    }

    pub fn sample_psi(&self) {
        if let Ok(report) = PsiMonitor::read_pressure() {
            self.inner.record_metric(
                AstrophageLevel::Debug,
                "psi-cpu",
                "CPU stall pressure",
                "avg10",
                report.cpu_some.avg10 as f64,
            );
            self.inner.record_metric(
                AstrophageLevel::Debug,
                "psi-memory",
                "Memory stall pressure",
                "avg10",
                report.memory_some.avg10 as f64,
            );
        }
    }
}

impl Default for AstrophageCoreLogger {
    fn default() -> Self {
        Self::new()
    }
}
