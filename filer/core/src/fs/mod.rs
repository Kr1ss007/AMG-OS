//! Filer Core File System Subsystem
//!
//! Indexes file system directories, extracts metadata, and reads folder contents.
//! Runs exclusively in Process 1 without UI dependencies.

use filer_protocol::FileEntry;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub struct FileSystemIndexer;

impl FileSystemIndexer {
    pub fn list_directory<P: AsRef<Path>>(dir_path: P) -> Result<Vec<FileEntry>, std::io::Error> {
        let path = dir_path.as_ref();
        let mut entries = Vec::new();

        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                let metadata = entry.metadata()?;

                let modified_timestamp_secs = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                entries.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    is_directory: file_type.is_dir(),
                    size_bytes: metadata.len(),
                    modified_timestamp_secs,
                });
            }
        }

        // Sort: directories first, then alphabetical by name
        entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }
}
