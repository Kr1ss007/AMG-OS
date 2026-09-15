//! astrophage-core: Process 1 Continuous Diagnostic Logger
//!
//! Maintains rolling circular telemetry buffer (10,000 entries) in Process 1.
//! Ingests real Linux kernel logs via non-blocking /dev/kmsg, samples Pressure Stall Information
//! (PSI) from /proc/pressure/{cpu,memory,io}, inspects NVMe storage health from sysfs,
//! and generates sanitized GitHub Issue diagnostic reports.

use amgos_protocol::ebus::{AstrophageLevel, AstrophageRecord};
use astrophage_protocol::GitHubIssuePayload;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::os::unix::io::FromRawFd;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_MAX_LOG_ENTRIES: usize = 10_000;
pub const KMSG_DEVICE_PATH: &str = "/dev/kmsg";
pub const OS_VERSION: &str = "AMGOS 0.0.1";
pub const OS_CODENAME: &str = "Upstream Color";

#[derive(Debug, Clone, PartialEq)]
pub struct PressureMetrics {
    pub avg10: f32,
    pub avg60: f32,
    pub avg300: f32,
    pub total_us: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemPsiReport {
    pub cpu_some: PressureMetrics,
    pub memory_some: PressureMetrics,
    pub memory_full: PressureMetrics,
    pub io_some: PressureMetrics,
    pub io_full: PressureMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NvmeStatReport {
    pub reads_completed: u64,
    pub sectors_read: u64,
    pub writes_completed: u64,
    pub sectors_written: u64,
    pub io_ticks_ms: u64,
}

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

    pub fn record_metric(&self, level: AstrophageLevel, subsystem: &str, message: &str, key: &str, value: f64) {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let record = AstrophageRecord {
            timestamp_ns,
            level,
            subsystem: subsystem.to_string(),
            message: message.to_string(),
            metric_key: Some(key.to_string()),
            metric_value: Some(value),
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

    pub fn get_by_subsystem(&self, subsystem: &str, limit: usize) -> Vec<AstrophageRecord> {
        let queue = self.entries.lock().unwrap();
        queue
            .iter()
            .filter(|r| r.subsystem == subsystem)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }
}

impl Default for AstrophageBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_LOG_ENTRIES)
    }
}

/// Linux Kernel /dev/kmsg Reader
///
/// Reads kernel log buffer in non-blocking mode with O_NONBLOCK | O_CLOEXEC.
/// Parses kernel priority levels and categorizes logs into AstrophageBuffer.
pub struct KmsgReader;

impl KmsgReader {
    pub fn read_available(max_lines: usize) -> Vec<AstrophageRecord> {
        let mut records = Vec::new();
        let path = Path::new(KMSG_DEVICE_PATH);
        if !path.exists() {
            return records;
        }

        unsafe {
            let c_path = std::ffi::CString::new(KMSG_DEVICE_PATH).unwrap();
            let fd = libc::open(c_path.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC);
            if fd < 0 {
                return records;
            }

            let file = std::fs::File::from_raw_fd(fd);
            let mut buf = [0u8; 4096];

            for _ in 0..max_lines {
                let bytes_read = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                if bytes_read <= 0 {
                    break;
                }

                if let Ok(line) = std::str::from_utf8(&buf[..bytes_read as usize]) {
                    if let Some(rec) = Self::parse_kmsg_entry(line) {
                        records.push(rec);
                    }
                }
            }

            // file will close fd upon dropping
            std::mem::forget(file);
            libc::close(fd);
        }

        records
    }

    pub fn parse_kmsg_entry(entry: &str) -> Option<AstrophageRecord> {
        // Format: <fac_pri>,<seq>,<ts_us>,<flags>;<msg>
        let (meta_part, msg_part) = entry.split_once(';')?;
        let mut meta_fields = meta_part.split(',');
        let pri_fac: u32 = meta_fields.next()?.parse().ok()?;
        let _seq: u64 = meta_fields.next()?.parse().ok()?;
        let ts_us: u64 = meta_fields.next()?.parse().ok()?;

        let priority = pri_fac & 7;
        let level = match priority {
            0..=2 => AstrophageLevel::Critical,
            3 => AstrophageLevel::Error,
            4 => AstrophageLevel::Warning,
            _ => AstrophageLevel::Info,
        };

        let timestamp_ns = ts_us * 1000;
        let message = msg_part.trim_end_matches(&['\r', '\n'][..]).to_string();

        Some(AstrophageRecord {
            timestamp_ns,
            level,
            subsystem: "kernel".to_string(),
            message,
            metric_key: None,
            metric_value: None,
        })
    }
}

/// Pressure Stall Information (PSI) Monitor
pub struct PsiMonitor;

impl PsiMonitor {
    pub fn read_pressure() -> io::Result<SystemPsiReport> {
        let cpu_some = Self::parse_psi_line("/proc/pressure/cpu", "some")?;
        let memory_some = Self::parse_psi_line("/proc/pressure/memory", "some")?;
        let memory_full = Self::parse_psi_line("/proc/pressure/memory", "full")?;
        let io_some = Self::parse_psi_line("/proc/pressure/io", "some")?;
        let io_full = Self::parse_psi_line("/proc/pressure/io", "full")?;

        Ok(SystemPsiReport {
            cpu_some,
            memory_some,
            memory_full,
            io_some,
            io_full,
        })
    }

    fn parse_psi_line(path_str: &str, prefix: &str) -> io::Result<PressureMetrics> {
        let content = fs::read_to_string(path_str)?;
        Self::parse_psi_text(&content, prefix)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse {}", path_str)))
    }

    pub fn parse_psi_text(text: &str, target_prefix: &str) -> Option<PressureMetrics> {
        for line in text.lines() {
            if line.starts_with(target_prefix) {
                let mut avg10 = 0.0f32;
                let mut avg60 = 0.0f32;
                let mut avg300 = 0.0f32;
                let mut total_us = 0u64;

                for token in line.split_whitespace().skip(1) {
                    if let Some((k, v)) = token.split_once('=') {
                        match k {
                            "avg10" => avg10 = v.parse().unwrap_or(0.0),
                            "avg60" => avg60 = v.parse().unwrap_or(0.0),
                            "avg300" => avg300 = v.parse().unwrap_or(0.0),
                            "total" => total_us = v.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }

                return Some(PressureMetrics {
                    avg10,
                    avg60,
                    avg300,
                    total_us,
                });
            }
        }
        None
    }
}

/// NVMe Storage Health Reader
pub struct StorageHealthMonitor;

impl StorageHealthMonitor {
    pub fn read_nvme_stat(device_name: &str) -> io::Result<NvmeStatReport> {
        let path = format!("/sys/block/{}/stat", device_name);
        let content = fs::read_to_string(&path)?;
        Self::parse_stat_line(&content)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid sysfs disk stat"))
    }

    pub fn parse_stat_line(text: &str) -> Option<NvmeStatReport> {
        let fields: Vec<&str> = text.split_whitespace().collect();
        if fields.len() >= 11 {
            let reads_completed = fields[0].parse().ok()?;
            let sectors_read = fields[2].parse().ok()?;
            let writes_completed = fields[4].parse().ok()?;
            let sectors_written = fields[6].parse().ok()?;
            let io_ticks_ms = fields[9].parse().ok()?;

            Some(NvmeStatReport {
                reads_completed,
                sectors_read,
                writes_completed,
                sectors_written,
                io_ticks_ms,
            })
        } else {
            None
        }
    }
}

/// Sanitized Diagnostic Issue Formatter
pub struct DiagnosticReportBuilder;

impl DiagnosticReportBuilder {
    pub fn build_github_issue(
        user_description: &str,
        buffer: &AstrophageBuffer,
    ) -> GitHubIssuePayload {
        let mut hardware_summary = String::new();

        // CPU info
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.starts_with("model name") {
                    if let Some((_, model)) = line.split_once(':') {
                        hardware_summary.push_str(model.trim());
                        break;
                    }
                }
            }
        }
        if hardware_summary.is_empty() {
            hardware_summary.push_str("x86_64 Platform");
        }

        // Memory info
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some((_, mem)) = line.split_once(':') {
                        hardware_summary.push_str(&format!(", RAM: {}", mem.trim()));
                        break;
                    }
                }
            }
        }

        // GPU info
        hardware_summary.push_str(", GPU: Discrete NVIDIA RTX 3050 + Intel UHD");

        // Collect recent errors and warnings
        let recent = buffer.get_recent(50);
        let timestamp_utc = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| format!("{} s since UNIX epoch", d.as_secs()))
            .unwrap_or_else(|_| "Unknown".to_string());

        GitHubIssuePayload {
            title: format!("[Diagnostic] Automated Telemetry Report — {}", OS_CODENAME),
            user_description: sanitize_text(user_description),
            hardware_summary,
            os_version: format!("{} ({})", OS_VERSION, OS_CODENAME),
            recent_event_log: recent,
            timestamp_utc,
        }
    }
}

/// Removes home directories and usernames to maintain strict privacy
fn sanitize_text(input: &str) -> String {
    let mut out = input.to_string();
    if let Ok(user) = std::env::var("USER") {
        if !user.is_empty() {
            out = out.replace(&format!("/home/{}", user), "/home/[USER]");
            out = out.replace(&user, "[USER]");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_astrophage_ring_buffer() {
        let buffer = AstrophageBuffer::new(5);
        for i in 0..10 {
            buffer.record(AstrophageLevel::Info, "test", &format!("Msg {}", i));
        }
        assert_eq!(buffer.len(), 5);
        let recent = buffer.get_recent(10);
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[4].message, "Msg 9");
        assert_eq!(recent[0].message, "Msg 5");
    }

    #[test]
    fn test_kmsg_entry_parser() {
        let line = "6,120,450012,-;pci 0000:01:00.0: NVIDIA RTX 3050 initialized";
        let rec = KmsgReader::parse_kmsg_entry(line).expect("Kmsg line should parse");
        assert_eq!(rec.level, AstrophageLevel::Info);
        assert_eq!(rec.subsystem, "kernel");
        assert_eq!(rec.timestamp_ns, 450012 * 1000);
        assert_eq!(rec.message, "pci 0000:01:00.0: NVIDIA RTX 3050 initialized");

        let err_line = "3,121,450020,-;nvme nvme0: Controller reset requested";
        let err_rec = KmsgReader::parse_kmsg_entry(err_line).expect("Err line should parse");
        assert_eq!(err_rec.level, AstrophageLevel::Error);
    }

    #[test]
    fn test_psi_parser() {
        let psi_sample = "some avg10=0.15 avg60=0.08 avg300=0.02 total=123456\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";
        let some = PsiMonitor::parse_psi_text(psi_sample, "some").expect("Should parse some");
        assert!((some.avg10 - 0.15).abs() < 0.001);
        assert!((some.avg60 - 0.08).abs() < 0.001);
        assert_eq!(some.total_us, 123456);

        let full = PsiMonitor::parse_psi_text(psi_sample, "full").expect("Should parse full");
        assert_eq!(full.total_us, 0);
    }

    #[test]
    fn test_real_psi_read_if_available() {
        if Path::new("/proc/pressure/cpu").exists() {
            let report = PsiMonitor::read_pressure().expect("Should read host PSI");
            assert!(report.cpu_some.total_us > 0);
        }
    }

    #[test]
    fn test_sanitized_github_issue_generation() {
        let buffer = AstrophageBuffer::new(10);
        buffer.record(AstrophageLevel::Warning, "thermal", "CPU Package temp 72C");
        let issue = DiagnosticReportBuilder::build_github_issue("Issue observed in /home/raven1zed/test", &buffer);
        assert!(issue.os_version.contains("Upstream Color"));
        assert!(!issue.user_description.contains("/home/raven1zed"));
        assert!(issue.user_description.contains("/home/[USER]"));
        let md = issue.to_markdown();
        assert!(md.contains("Upstream Color"));
        assert!(md.contains("CPU Package temp 72C"));
    }
}
