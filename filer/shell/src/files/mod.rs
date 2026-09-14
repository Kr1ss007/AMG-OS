//! File Browser Navigation Panels (Process 2 Shell)
//!
//! Presents directory listings, breadcrumb navigation, and file item selection.

use filer_protocol::FileEntry;

#[derive(Debug, Clone)]
pub struct FileBrowserPanel {
    pub current_directory: String,
    pub entries: Vec<FileEntry>,
    pub selected_index: Option<usize>,
}

impl FileBrowserPanel {
    pub fn new(initial_dir: &str) -> Self {
        Self {
            current_directory: initial_dir.to_string(),
            entries: Vec::new(),
            selected_index: None,
        }
    }

    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.selected_index = None;
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.selected_index.and_then(|i| self.entries.get(i))
    }
}
