//! filer-shell: Process 2 Frontend UI for Filer & Pathfinder
//!
//! Owns Pathfinder search bar UI, Zen Browser embed container,
//! dual-pane file browser, and symmetric install/uninstall dialogs.

pub mod browser;
pub mod files;
pub mod installer;
pub mod ipc;
pub mod pathfinder;

pub use browser::ZenEmbedView;
pub use files::FileBrowserPanel;
pub use installer::{PackageActionDialog, PackageActionType};
pub use pathfinder::PathfinderSearchBar;
