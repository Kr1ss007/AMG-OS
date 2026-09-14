//! Pathfinder Search Bar UI State (Process 2 Shell)
//!
//! Owns query input state, debounced request dispatching over e-bus,
//! and unified categorization across files, applications, settings, and web.

use amgos_protocol::ebus::{PathfinderCategory, PathfinderItem};

#[derive(Debug, Clone)]
pub struct PathfinderSearchBar {
    pub input_query: String,
    pub is_active: bool,
    pub selected_index: usize,
    pub active_results: Vec<PathfinderItem>,
}

impl PathfinderSearchBar {
    pub fn new() -> Self {
        Self {
            input_query: String::new(),
            is_active: false,
            selected_index: 0,
            active_results: Vec::new(),
        }
    }

    pub fn set_query(&mut self, query: &str) {
        self.input_query = query.to_string();
        self.is_active = !query.is_empty();
        self.selected_index = 0;
    }

    pub fn set_results(&mut self, results: Vec<PathfinderItem>) {
        self.active_results = results;
        self.selected_index = 0;
    }

    pub fn select_next(&mut self) {
        if !self.active_results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.active_results.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.active_results.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.active_results.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn current_selected_item(&self) -> Option<&PathfinderItem> {
        self.active_results.get(self.selected_index)
    }

    pub fn results_by_category(&self, cat: PathfinderCategory) -> Vec<&PathfinderItem> {
        self.active_results
            .iter()
            .filter(|item| item.category == cat)
            .collect()
    }
}

impl Default for PathfinderSearchBar {
    fn default() -> Self {
        Self::new()
    }
}
