//! Download Manager Subsystem (Process 1)
//!
//! Handles asynchronous package downloads to an isolated staging area.

use std::path::{Path, PathBuf};

pub struct DownloadManager {
    staging_dir: PathBuf,
}

impl DownloadManager {
    pub fn new<P: AsRef<Path>>(staging_dir: P) -> Self {
        Self {
            staging_dir: staging_dir.as_ref().to_path_buf(),
        }
    }

    pub fn stage_path(&self, file_name: &str) -> PathBuf {
        self.staging_dir.join(file_name)
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new("/tmp/amgos-staging")
    }
}
