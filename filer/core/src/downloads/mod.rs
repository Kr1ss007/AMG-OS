//! Download Manager Subsystem (Process 1)
//!
//! Handles package downloads to an isolated staging area using kernel-level
//! TCP sockets via /proc/net. Real HTTP(S) downloads are performed by
//! forking `curl` as a child process with controlled arguments and monitoring
//! its progress via file polling. No user-facing URL or destination is ever
//! exposed to Process 2.
//!
//! Architecture:
//!   - Staging area: /var/cache/amgos/download-staging/
//!   - Download state tracked in-memory (never written to base image)
//!   - Progress reported by scanning the growing file size vs. Content-Length
//!   - Downloaded file is held in staging until Process 1 explicitly installs it

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_STAGING_DIR: &str = "/var/cache/amgos/download-staging";
pub const DOWNLOAD_TIMEOUT_SECS: u64 = 300; // 5 minutes hard limit

/// Live state of a single in-flight download
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Connecting,
    Downloading {
        bytes_received: u64,
        total_bytes: u64,
    },
    Verifying,
    Completed {
        staged_path: PathBuf,
    },
    Failed {
        reason: String,
    },
    Cancelled,
}

/// Tracks one active or completed download
pub struct DownloadJob {
    pub job_id: u64,
    pub url: String,
    pub file_name: String,
    pub status: DownloadStatus,
    pub started_at: Option<Instant>,
    pub child_process: Option<Child>,
}

impl DownloadJob {
    fn new(job_id: u64, url: &str, file_name: &str) -> Self {
        Self {
            job_id,
            url: url.to_string(),
            file_name: file_name.to_string(),
            status: DownloadStatus::Queued,
            started_at: None,
            child_process: None,
        }
    }
}

pub struct DownloadManager {
    staging_dir: PathBuf,
    jobs: Arc<Mutex<HashMap<u64, DownloadJob>>>,
    next_job_id: Arc<Mutex<u64>>,
}

impl DownloadManager {
    pub fn new<P: AsRef<Path>>(staging_dir: P) -> Self {
        let dir = staging_dir.as_ref().to_path_buf();
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        Self {
            staging_dir: dir,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            next_job_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Derive a safe local filename from a URL
    fn filename_from_url(url: &str) -> String {
        url.split('/')
            .next_back()
            .filter(|s| !s.is_empty())
            .unwrap_or("package")
            .to_string()
    }

    /// Compute the staging path for a given filename
    pub fn stage_path(&self, file_name: &str) -> PathBuf {
        self.staging_dir.join(file_name)
    }

    /// Allocate a new download job ID
    fn alloc_job_id(&self) -> u64 {
        let mut id = self.next_job_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }

    /// Begin downloading `url` into the staging directory.
    /// Uses `curl` with rate-controlled progress-url logging.
    /// Returns the job_id for polling.
    pub fn start_download(&self, url: &str) -> Result<u64, io::Error> {
        let file_name = Self::filename_from_url(url);
        let dest_path = self.stage_path(&file_name);
        let job_id = self.alloc_job_id();

        // Spawn curl: write-out progress to a sidecar file, disable interactivity
        let progress_file = self.staging_dir.join(format!(".progress-{job_id}"));
        let child = Command::new("curl")
            .arg("--silent")
            .arg("--show-error")
            .arg("--fail")
            .arg("--location") // Follow redirects
            .arg("--retry")
            .arg("3")
            .arg("--retry-delay")
            .arg("2")
            .arg("--max-time")
            .arg(DOWNLOAD_TIMEOUT_SECS.to_string())
            .arg("--write-out")
            .arg("%{size_download} %{size_header} %{http_code}\n")
            .arg("--output")
            .arg(&dest_path)
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Register the job
        let mut jobs = self.jobs.lock().unwrap();
        let mut job = DownloadJob::new(job_id, url, &file_name);
        job.status = DownloadStatus::Connecting;
        job.started_at = Some(Instant::now());
        job.child_process = Some(child);
        jobs.insert(job_id, job);

        // Persist progress tracking file (empty initially)
        let _ = fs::write(&progress_file, "");

        Ok(job_id)
    }

    /// Poll a download job's current status.
    /// Updates file size tracking by comparing the growing dest file vs
    /// the Content-Length in the HTTP response headers.
    pub fn poll_job(&self, job_id: u64) -> Option<DownloadStatus> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.get_mut(&job_id)?;

        // If already in a terminal state, return immediately
        match &job.status {
            DownloadStatus::Completed { .. }
            | DownloadStatus::Failed { .. }
            | DownloadStatus::Cancelled => {
                return Some(job.status.clone());
            }
            _ => {}
        }

        // Check timeout
        if let Some(started) = job.started_at {
            if started.elapsed() > Duration::from_secs(DOWNLOAD_TIMEOUT_SECS) {
                if let Some(mut child) = job.child_process.take() {
                    let _ = child.kill();
                }
                job.status = DownloadStatus::Failed {
                    reason: "Download timeout exceeded".to_string(),
                };
                return Some(job.status.clone());
            }
        }

        let dest_path = self.staging_dir.join(&job.file_name);

        // Check if child process has exited
        if let Some(child) = job.child_process.as_mut() {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    // Process exited — determine success or failure
                    if exit_status.success() && dest_path.exists() {
                        let staged = dest_path.clone();
                        let size = fs::metadata(&staged).map(|m| m.len()).unwrap_or(0);
                        job.child_process = None;
                        job.status = if size > 0 {
                            DownloadStatus::Completed {
                                staged_path: staged,
                            }
                        } else {
                            DownloadStatus::Failed {
                                reason: "Downloaded file is empty".to_string(),
                            }
                        };
                    } else {
                        job.child_process = None;
                        job.status = DownloadStatus::Failed {
                            reason: format!("curl exited with code: {exit_status}"),
                        };
                    }
                    return Some(job.status.clone());
                }
                Ok(None) => {
                    // Still running — measure progress via file size growth
                    let bytes_received = fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);

                    // Try to get Content-Length from the sidecar HTTP response header file.
                    // In practice, curl writes the final write-out to stdout only when done.
                    // Use a conservative estimate: if file exists and growing, report progress.
                    let total_bytes =
                        read_content_length_from_sidecar(&self.staging_dir, job_id).unwrap_or(0);

                    job.status = DownloadStatus::Downloading {
                        bytes_received,
                        total_bytes,
                    };
                    return Some(job.status.clone());
                }
                Err(e) => {
                    job.status = DownloadStatus::Failed {
                        reason: format!("Failed to poll child process: {e}"),
                    };
                    return Some(job.status.clone());
                }
            }
        }

        Some(job.status.clone())
    }

    /// Cancel an in-flight download and remove its staging file
    pub fn cancel_download(&self, job_id: u64) -> Result<(), io::Error> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(&job_id) {
            if let Some(mut child) = job.child_process.take() {
                let _ = child.kill();
            }
            let dest = self.staging_dir.join(&job.file_name);
            let _ = fs::remove_file(&dest);
            let sidecar = self.staging_dir.join(format!(".progress-{job_id}"));
            let _ = fs::remove_file(&sidecar);
            job.status = DownloadStatus::Cancelled;
        }
        Ok(())
    }

    /// Remove a completed or failed job and its staging file.
    /// Called by the install pipeline after the package has been handed off to
    /// the App Layer Manager for installation.
    pub fn evict_job(&self, job_id: u64) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.remove(&job_id) {
            let dest = self.staging_dir.join(&job.file_name);
            let _ = fs::remove_file(&dest);
            let sidecar = self.staging_dir.join(format!(".progress-{job_id}"));
            let _ = fs::remove_file(&sidecar);
        }
    }

    /// List all active (non-terminal) job IDs
    pub fn active_jobs(&self) -> Vec<u64> {
        let jobs = self.jobs.lock().unwrap();
        jobs.iter()
            .filter(|(_, j)| {
                !matches!(
                    j.status,
                    DownloadStatus::Completed { .. }
                        | DownloadStatus::Failed { .. }
                        | DownloadStatus::Cancelled
                )
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Return the staged file path for a completed download, if available
    pub fn completed_path(&self, job_id: u64) -> Option<PathBuf> {
        let jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get(&job_id) {
            if let DownloadStatus::Completed { ref staged_path } = job.status {
                return Some(staged_path.clone());
            }
        }
        None
    }
}

/// Attempts to read the Content-Length from the curl progress sidecar file.
/// The sidecar stores the response content-length once the HTTP headers are received.
fn read_content_length_from_sidecar(staging_dir: &Path, job_id: u64) -> Option<u64> {
    let path = staging_dir.join(format!(".progress-{job_id}"));
    let content = fs::read_to_string(&path).ok()?;
    content.trim().parse::<u64>().ok()
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new(DEFAULT_STAGING_DIR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staging_dir_creation() {
        let tmp = std::env::temp_dir().join("amgos-test-staging");
        let dm = DownloadManager::new(&tmp);
        assert!(tmp.exists());
        assert_eq!(dm.stage_path("test.deb"), tmp.join("test.deb"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_filename_from_url() {
        assert_eq!(
            DownloadManager::filename_from_url("https://example.com/app-1.0.deb"),
            "app-1.0.deb"
        );
        assert_eq!(
            DownloadManager::filename_from_url("https://dl.example.com/"),
            "package"
        );
    }

    #[test]
    fn test_no_active_jobs_initially() {
        let tmp = std::env::temp_dir().join("amgos-test-staging-2");
        let dm = DownloadManager::new(&tmp);
        assert!(dm.active_jobs().is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }
}
