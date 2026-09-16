//! Filer Core File System Subsystem
//!
//! Indexes file system directories, extracts metadata, reads folder contents,
//! and continuously monitors filesystem changes using Linux inotify.
//! Runs exclusively in Process 1 without UI dependencies.

use filer_protocol::FileEntry;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct FileSystemIndexer;

impl FileSystemIndexer {
    pub fn list_directory<P: AsRef<Path>>(dir_path: P) -> Result<Vec<FileEntry>, io::Error> {
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

    /// Recursively scan directories up to a depth limit, collecting file entries
    pub fn scan_recursive<P: AsRef<Path>>(
        root: P,
        max_depth: usize,
    ) -> Result<Vec<FileEntry>, io::Error> {
        let mut results = Vec::new();
        Self::scan_recursive_internal(root.as_ref(), 0, max_depth, &mut results)?;
        Ok(results)
    }

    /// Check whether a path is protected by AMGOS rm -rf prohibition (SPEC Section 2.2 & Week 4 Day 4)
    pub fn is_protected_critical_path<P: AsRef<Path>>(target: P) -> bool {
        let path = target.as_ref();
        let path_str = path.to_string_lossy();
        if path_str == "/" || path_str.is_empty() {
            return true;
        }
        const CRITICAL_ROOTS: &[&str] = &[
            "/boot",
            "/usr",
            "/etc",
            "/lib",
            "/lib64",
            "/bin",
            "/sbin",
            "/dev",
            "/proc",
            "/sys",
            "/var",
            "/home",
            "/root",
            "/mnt/base_a",
            "/mnt/base_b",
            "/opt",
        ];
        for &crit in CRITICAL_ROOTS {
            if path_str == crit {
                return true;
            }
            if matches!(
                crit,
                "/boot"
                    | "/usr"
                    | "/etc"
                    | "/lib"
                    | "/lib64"
                    | "/bin"
                    | "/sbin"
                    | "/dev"
                    | "/proc"
                    | "/sys"
                    | "/mnt/base_a"
                    | "/mnt/base_b"
            ) && path_str.starts_with(&format!("{crit}/"))
            {
                return true;
            }
        }
        false
    }

    /// Safely delete a target path while strictly preventing deletion of critical OS hierarchies
    pub fn safe_delete<P: AsRef<Path>>(target: P) -> Result<(), io::Error> {
        let path = target.as_ref();
        if Self::is_protected_critical_path(path) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Prohibited operation: critical system path '{}' cannot be deleted",
                    path.display()
                ),
            ));
        }
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else if path.exists() {
            fs::remove_file(path)
        } else {
            Ok(())
        }
    }

    fn scan_recursive_internal(
        dir: &Path,
        current_depth: usize,
        max_depth: usize,
        accumulator: &mut Vec<FileEntry>,
    ) -> Result<(), io::Error> {
        if current_depth > max_depth || !dir.is_dir() {
            return Ok(());
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_type = match entry.file_type() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let is_dir = file_type.is_dir();
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                accumulator.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: path.to_string_lossy().to_string(),
                    is_directory: is_dir,
                    size_bytes: metadata.len(),
                    modified_timestamp_secs: modified,
                });

                if is_dir && current_depth < max_depth {
                    let _ = Self::scan_recursive_internal(
                        &path,
                        current_depth + 1,
                        max_depth,
                        accumulator,
                    );
                }
            }
        }

        Ok(())
    }
}

/// Filesystem event emitted by inotify watcher
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

/// Continuous Linux inotify directory watcher
pub struct InotifyWatcher {
    fd: i32,
    watch_descriptors: HashMap<i32, PathBuf>,
}

impl InotifyWatcher {
    pub fn new() -> Result<Self, io::Error> {
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            fd,
            watch_descriptors: HashMap::new(),
        })
    }

    /// Add a directory path to the active inotify watch set
    pub fn add_watch<P: AsRef<Path>>(&mut self, dir_path: P) -> Result<i32, io::Error> {
        let path = dir_path.as_ref();
        let c_path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

        let mask = libc::IN_CREATE
            | libc::IN_DELETE
            | libc::IN_MODIFY
            | libc::IN_MOVED_FROM
            | libc::IN_MOVED_TO;

        let wd = unsafe { libc::inotify_add_watch(self.fd, c_path.as_ptr(), mask) };
        if wd < 0 {
            return Err(io::Error::last_os_error());
        }

        self.watch_descriptors.insert(wd, path.to_path_buf());
        Ok(wd)
    }

    /// Poll for pending filesystem events without blocking
    pub fn poll_events(&self) -> Result<Vec<FsEvent>, io::Error> {
        let mut events = Vec::new();
        let mut buffer = [0u8; 4096];

        let bytes_read = unsafe {
            libc::read(
                self.fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
            )
        };

        if bytes_read < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                return Ok(events);
            }
            return Err(err);
        }

        let mut offset = 0;
        while offset < bytes_read as usize {
            let event_ptr = unsafe { buffer.as_ptr().add(offset) as *const libc::inotify_event };
            let inotify_evt = unsafe { &*event_ptr };

            let wd = inotify_evt.wd;
            let mask = inotify_evt.mask;
            let len = inotify_evt.len as usize;

            let file_name = if len > 0 {
                let name_ptr = unsafe {
                    buffer
                        .as_ptr()
                        .add(offset + std::mem::size_of::<libc::inotify_event>())
                };
                let c_str = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const libc::c_char) };
                c_str.to_string_lossy().to_string()
            } else {
                String::new()
            };

            if let Some(parent_path) = self.watch_descriptors.get(&wd) {
                let target_path = if file_name.is_empty() {
                    parent_path.clone()
                } else {
                    parent_path.join(&file_name)
                };

                if mask & (libc::IN_CREATE | libc::IN_MOVED_TO) != 0 {
                    events.push(FsEvent::Created(target_path));
                } else if mask & (libc::IN_DELETE | libc::IN_MOVED_FROM) != 0 {
                    events.push(FsEvent::Deleted(target_path));
                } else if mask & libc::IN_MODIFY != 0 {
                    events.push(FsEvent::Modified(target_path));
                }
            }

            offset += std::mem::size_of::<libc::inotify_event>() + len;
        }

        Ok(events)
    }
}

impl Drop for InotifyWatcher {
    fn drop(&mut self) {
        for &wd in self.watch_descriptors.keys() {
            unsafe {
                libc::inotify_rm_watch(self.fd, wd);
            }
        }
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_list_directory() {
        let entries =
            FileSystemIndexer::list_directory(".").expect("Failed to list current directory");
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_inotify_watcher() {
        let tmp_dir = std::env::temp_dir().join("amgos_inotify_test");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).expect("Failed to create test dir");

        let mut watcher = InotifyWatcher::new().expect("Failed to init inotify");
        watcher.add_watch(&tmp_dir).expect("Failed to add watch");

        // Create a test file
        let test_file = tmp_dir.join("test_signal.txt");
        let mut f = File::create(&test_file).expect("Failed to create test file");
        f.write_all(b"AMGOS inotify test")
            .expect("Failed to write test file");
        f.sync_all().expect("Sync failed");

        // Poll events
        std::thread::sleep(std::time::Duration::from_millis(100));
        let events = watcher.poll_events().expect("Failed to poll events");
        assert!(!events.is_empty(), "Inotify should capture file creation");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_rm_rf_prohibition() {
        assert!(FileSystemIndexer::is_protected_critical_path("/"));
        assert!(FileSystemIndexer::is_protected_critical_path("/usr"));
        assert!(FileSystemIndexer::is_protected_critical_path("/usr/bin"));
        assert!(FileSystemIndexer::is_protected_critical_path(
            "/etc/systemd"
        ));
        assert!(FileSystemIndexer::is_protected_critical_path("/boot"));
        assert!(FileSystemIndexer::is_protected_critical_path("/boot/efi"));
        assert!(FileSystemIndexer::is_protected_critical_path("/mnt/base_a"));

        assert!(!FileSystemIndexer::is_protected_critical_path(
            "/tmp/test_dir"
        ));
        assert!(!FileSystemIndexer::is_protected_critical_path(
            "/home/user/document.txt"
        ));

        // Attempting to delete protected path must return PermissionDenied
        let res = FileSystemIndexer::safe_delete("/usr");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
}
