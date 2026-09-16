//! Pathfinder Query Resolution & Inverted Search Index
//!
//! Evaluates search queries across Applications, Files, Settings, and Web.
//! Ranks results using exact match, prefix match, and token scoring.
//! Supports live filesystem indexing, system application parsing, and symmetric removal.

use amgos_protocol::ebus::{PathfinderCategory, PathfinderItem};
use filer_protocol::FileEntry;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

        // 1. Seed First-Class Native Citizen Apps
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
                "pathfinder".into(),
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
                "gecko".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "app-terminow".into(),
            title: "Terminow".into(),
            description: "GPU-accelerated terminal emulator (JetBrains Mono)".into(),
            category: PathfinderCategory::Application,
            action_uri: "amgos://apps/terminow".into(),
            is_installed: true,
            tokens: vec![
                "terminow".into(),
                "terminal".into(),
                "cli".into(),
                "bash".into(),
                "shell".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "app-settings".into(),
            title: "Settings".into(),
            description: "Single configuration surface for AMG-OS".into(),
            category: PathfinderCategory::Application,
            action_uri: "amgos://apps/settings".into(),
            is_installed: true,
            tokens: vec![
                "settings".into(),
                "config".into(),
                "preferences".into(),
                "control".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "app-astrophage".into(),
            title: "Astrophage".into(),
            description: "Hardware and system diagnostic reporter".into(),
            category: PathfinderCategory::Application,
            action_uri: "amgos://apps/astrophage".into(),
            is_installed: true,
            tokens: vec![
                "astrophage".into(),
                "diagnostics".into(),
                "hardware".into(),
                "bugs".into(),
                "report".into(),
            ],
        });

        // 2. Seed Settings Topics
        index.add_entry(SearchDocument {
            id: "setting-power".into(),
            title: "Power & Battery Settings".into(),
            description: "Pre-configured power profiles: Endurance, Balanced, MAX".into(),
            category: PathfinderCategory::Setting,
            action_uri: "amgos://settings/power".into(),
            is_installed: true,
            tokens: vec![
                "power".into(),
                "battery".into(),
                "endurance".into(),
                "balanced".into(),
                "max".into(),
                "acpi".into(),
                "performance".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "setting-display".into(),
            title: "Display Settings".into(),
            description: "Resolution, 144Hz refresh rate, and scaling factor".into(),
            category: PathfinderCategory::Setting,
            action_uri: "amgos://settings/display".into(),
            is_installed: true,
            tokens: vec![
                "display".into(),
                "monitor".into(),
                "resolution".into(),
                "scale".into(),
                "144hz".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "setting-sound".into(),
            title: "Sound & Audio Settings".into(),
            description: "Audio-Video Manager (AVM) routing, F3 boot chime, and ducking".into(),
            category: PathfinderCategory::Setting,
            action_uri: "amgos://settings/sound".into(),
            is_installed: true,
            tokens: vec![
                "sound".into(),
                "audio".into(),
                "volume".into(),
                "avm".into(),
                "chime".into(),
                "pipewire".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "setting-touchpad".into(),
            title: "Touchpad & Gestures".into(),
            description: "MotionWave gestures, scroll direction, and sensitivity".into(),
            category: PathfinderCategory::Setting,
            action_uri: "amgos://settings/touchpad".into(),
            is_installed: true,
            tokens: vec![
                "touchpad".into(),
                "gestures".into(),
                "motionwave".into(),
                "swipe".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "setting-network".into(),
            title: "Network & Wi-Fi Settings".into(),
            description: "Connection management and Process 1 credential vault".into(),
            category: PathfinderCategory::Setting,
            action_uri: "amgos://settings/network".into(),
            is_installed: true,
            tokens: vec![
                "network".into(),
                "wifi".into(),
                "internet".into(),
                "ethernet".into(),
                "credentials".into(),
            ],
        });

        // 3. Seed Installable Web Apps (Available in Filer/Pathfinder)
        index.add_entry(SearchDocument {
            id: "web-obs".into(),
            title: "OBS Studio".into(),
            description: "Open Broadcaster Software for video recording and live streaming".into(),
            category: PathfinderCategory::InstallableWeb,
            action_uri: "amgos://install/obs-studio".into(),
            is_installed: false,
            tokens: vec![
                "obs".into(),
                "stream".into(),
                "record".into(),
                "video".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "web-vlc".into(),
            title: "VLC Media Player".into(),
            description: "Universal multimedia player and streaming framework".into(),
            category: PathfinderCategory::InstallableWeb,
            action_uri: "amgos://install/vlc".into(),
            is_installed: false,
            tokens: vec![
                "vlc".into(),
                "video".into(),
                "media".into(),
                "player".into(),
            ],
        });

        index.add_entry(SearchDocument {
            id: "web-blender".into(),
            title: "Blender".into(),
            description: "Professional 3D creation suite: modeling, animation, rendering".into(),
            category: PathfinderCategory::InstallableWeb,
            action_uri: "amgos://install/blender".into(),
            is_installed: false,
            tokens: vec![
                "blender".into(),
                "3d".into(),
                "render".into(),
                "animation".into(),
            ],
        });

        index
    }

    pub fn add_entry(&mut self, doc: SearchDocument) {
        // If entry with same ID exists, update it
        if let Some(pos) = self.documents.iter().position(|d| d.id == doc.id) {
            self.documents[pos] = doc;
            self.rebuild_token_map();
            return;
        }

        let doc_idx = self.documents.len();
        for token in &doc.tokens {
            self.token_map
                .entry(token.to_lowercase())
                .or_default()
                .push(doc_idx);
        }
        self.documents.push(doc);
    }

    /// Dynamically index a file discovered on the filesystem
    pub fn index_file_entry(&mut self, file: &FileEntry) {
        let title = file.name.clone();
        let tokens = tokenize_string(&title);
        let id = format!("file:{}", file.path);

        self.add_entry(SearchDocument {
            id,
            title,
            description: format!("File at {}", file.path),
            category: PathfinderCategory::File,
            action_uri: format!("amgos://open?path={}", file.path),
            is_installed: true,
            tokens,
        });
    }

    /// Symmetric removal: remove app or file from index immediately upon uninstall or delete
    pub fn remove_entry(&mut self, id: &str) -> bool {
        if let Some(pos) = self.documents.iter().position(|d| d.id == id) {
            self.documents.remove(pos);
            self.rebuild_token_map();
            true
        } else {
            false
        }
    }

    fn rebuild_token_map(&mut self) {
        self.token_map.clear();
        for (idx, doc) in self.documents.iter().enumerate() {
            for token in &doc.tokens {
                self.token_map
                    .entry(token.to_lowercase())
                    .or_default()
                    .push(idx);
            }
        }
    }

    /// Scan installed system .desktop files from standard directories
    pub fn scan_system_applications(&mut self) {
        let app_dirs = [
            Path::new("/usr/share/applications"),
            Path::new("/var/lib/amgos/apps"),
        ];

        for dir in app_dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "desktop") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Some(doc) = parse_desktop_file(&path, &content) {
                                self.add_entry(doc);
                            }
                        }
                    }
                }
            }
        }
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
                score += 60.0; // Prefix match
            } else if title_lower.contains(&q) {
                score += 30.0; // Substring match
            }

            if desc_lower.contains(&q) {
                score += 15.0;
            }

            for token in &doc.tokens {
                let token_lower = token.to_lowercase();
                if token_lower == q {
                    score += 40.0;
                } else if token_lower.starts_with(&q) {
                    score += 20.0;
                }
            }

            if score > 0.0 {
                // Installed apps get slight priority over web installables
                if doc.is_installed {
                    score += 5.0;
                }
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

fn tokenize_string(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_lowercase())
        .collect()
}

fn parse_desktop_file(path: &Path, content: &str) -> Option<SearchDocument> {
    let mut name: Option<String> = None;
    let mut comment: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut keywords = Vec::new();
    let mut is_nodisplay = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("NoDisplay=true") {
            is_nodisplay = true;
        } else if line.starts_with("Name=") && name.is_none() {
            name = Some(line.trim_start_matches("Name=").trim().to_string());
        } else if line.starts_with("Comment=") && comment.is_none() {
            comment = Some(line.trim_start_matches("Comment=").trim().to_string());
        } else if line.starts_with("Exec=") && exec.is_none() {
            exec = Some(line.trim_start_matches("Exec=").trim().to_string());
        } else if line.starts_with("Keywords=") {
            let kw_str = line.trim_start_matches("Keywords=").trim();
            for kw in kw_str.split(';') {
                let clean = kw.trim();
                if !clean.is_empty() {
                    keywords.push(clean.to_lowercase());
                }
            }
        }
    }

    if is_nodisplay || name.is_none() {
        return None;
    }

    let title = name.unwrap();
    let description = comment.unwrap_or_else(|| format!("Application {title}"));
    let mut tokens = tokenize_string(&title);
    tokens.extend(keywords);

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| title.to_lowercase());
    let id = format!("app-desktop-{stem}");

    Some(SearchDocument {
        id,
        title: title.clone(),
        description,
        category: PathfinderCategory::Application,
        action_uri: format!("amgos://apps/{stem}"),
        is_installed: true,
        tokens,
    })
}

impl Default for PathfinderIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathfinder_query_ranking() {
        let index = PathfinderIndex::new();
        let results = index.query("power", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].category, PathfinderCategory::Setting);
        assert!(results[0].title.contains("Power"));

        let zen_results = index.query("zen", 5);
        assert!(!zen_results.is_empty());
        assert_eq!(zen_results[0].id, "app-zen");
        assert!(zen_results[0].is_installed);
    }

    #[test]
    fn test_pathfinder_symmetric_removal() {
        let mut index = PathfinderIndex::new();
        assert!(index.remove_entry("app-terminow"));
        let results = index.query("terminow", 5);
        assert!(
            results.is_empty(),
            "Terminow should be gone from Pathfinder immediately"
        );
    }
}
