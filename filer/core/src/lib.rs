//! filer-core: Process 1 Backend for Filer & Pathfinder
//!
//! Owns filesystem indexing, Pathfinder search index, package format inspection,
//! and download staging. Runs with zero UI dependencies.

pub mod downloads;
pub mod fs;
pub mod ipc;
pub mod packages;
pub mod pathfinder;

pub use downloads::DownloadManager;
pub use fs::{FileSystemIndexer, InotifyWatcher};
pub use packages::{AppLayerManager, PackageInspector};
pub use pathfinder::{PathfinderIndex, SearchDocument};
