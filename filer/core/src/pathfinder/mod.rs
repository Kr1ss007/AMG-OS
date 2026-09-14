//! Pathfinder Query Resolution & Inverted Search Index
//!
//! Evaluates search queries across Applications, Files, Settings, and Web.
//! Ranks results using exact match, prefix match, and substring scoring.

use amgos_protocol::ebus::{PathfinderCategory, PathfinderItem};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: PathfinderCategory,
    pub action_uri: String,
    pub is_installed: bool,
    pub tokens: Vec<String>,
}

pub struct PathfinderIndex {
    documents: Vec<SearchDocument>,
    token_map: HashMap<String, Vec<usize>>,
}

impl PathfinderIndex {
    pub fn new() -> Self {
        let mut index = Self {
            documents: Vec::new(),
            token_map: HashMap::new(),
        };

        // Seed core system targets
        index.add_entry(SearchDocument {
            id: "app-filer".into(),
            title: "Filer".into(),
            description: "Primary file navigation and application discovery".into(),
            category: PathfinderCategory::Application,
            action_uri: "amgos://apps/filer".into(),
            is_installed: true,
            tokens: vec![
                "filer".into(),
                "files".into(),
                "finder".into(),
                "explorer".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "app-zen".into(),
            title: "Zen Browser".into(),
            description: "Native privacy browser powered by Gecko".into(),
            category: PathfinderCategory::Application,
            action_uri: "amgos://apps/zen".into(),
            is_installed: true,
            tokens: vec![
                "zen".into(),
                "browser".into(),
                "web".into(),
                "internet".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "app-terminow".into(),
            title: "Terminow".into(),
            description: "GPU-accelerated terminal emulator".into(),
            category: PathfinderCategory::Application,
            action_uri: "amgos://apps/terminow".into(),
            is_installed: true,
            tokens: vec![
                "terminow".into(),
                "terminal".into(),
                "cli".into(),
                "bash".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "setting-display".into(),
            title: "Display Settings".into(),
            description: "Resolution, refresh rate (144Hz), and scaling factor".into(),
            category: PathfinderCategory::Setting,
            action_uri: "amgos://settings/display".into(),
            is_installed: true,
            tokens: vec![
                "display".into(),
                "monitor".into(),
                "resolution".into(),
                "scale".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "setting-sound".into(),
            title: "Sound & Audio Settings".into(),
            description: "Audio-Video Manager stream routing and volume controls".into(),
            category: PathfinderCategory::Setting,
            action_uri: "amgos://settings/sound".into(),
            is_installed: true,
            tokens: vec![
                "sound".into(),
                "audio".into(),
                "volume".into(),
                "avm".into(),
                "chime".into(),
            ],
        });

        index
    }

    pub fn add_entry(&mut self, doc: SearchDocument) {
        let doc_idx = self.documents.len();
        for token in &doc.tokens {
            self.token_map
                .entry(token.to_lowercase())
                .or_default()
                .push(doc_idx);
        }
        self.documents.push(doc);
    }

    /// Query the index and return ranked results
    pub fn query(&self, query_text: &str, limit: usize) -> Vec<PathfinderItem> {
        let q = query_text.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        let mut scores: HashMap<usize, f32> = HashMap::new();

        for (idx, doc) in self.documents.iter().enumerate() {
            let title_lower = doc.title.to_lowercase();
            let desc_lower = doc.description.to_lowercase();

            let mut score = 0.0;
            if title_lower == q {
                score += 100.0; // Exact match
            } else if title_lower.starts_with(&q) {
                score += 50.0; // Prefix match
            } else if title_lower.contains(&q) {
                score += 25.0; // Substring match
            }

            if desc_lower.contains(&q) {
                score += 10.0;
            }

            for token in &doc.tokens {
                if token == &q {
                    score += 40.0;
                } else if token.starts_with(&q) {
                    score += 20.0;
                }
            }

            if score > 0.0 {
                scores.insert(idx, score);
            }
        }

        let mut ranked: Vec<(usize, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        ranked
            .into_iter()
            .take(limit)
            .map(|(idx, score)| {
                let doc = &self.documents[idx];
                PathfinderItem {
                    id: doc.id.clone(),
                    title: doc.title.clone(),
                    description: doc.description.clone(),
                    category: doc.category,
                    score,
                    action_uri: doc.action_uri.clone(),
                    is_installed: doc.is_installed,
                }
            })
            .collect()
    }
}

impl Default for PathfinderIndex {
    fn default() -> Self {
        Self::new()
    }
}
