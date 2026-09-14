//! Zen Browser Embed Container (Process 2 Shell)
//!
//! Wraps embedded Gecko/Zen web rendering engine inside Filer.
//! Intercepts downloads of installable packages and delegates them to Process 1.

#[derive(Debug, Clone)]
pub struct ZenEmbedView {
    pub current_url: String,
    pub page_title: String,
    pub is_loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

impl ZenEmbedView {
    pub fn new() -> Self {
        Self {
            current_url: "amgos://start".to_string(),
            page_title: "AMG-OS Start".to_string(),
            is_loading: false,
            can_go_back: false,
            can_go_forward: false,
        }
    }

    pub fn navigate_to(&mut self, url: &str) {
        self.current_url = url.to_string();
        self.is_loading = true;
    }

    /// Check if target URL points to an installable package (.deb, .flatpak, .appimage, .snap)
    pub fn is_installable_package_url(&self, url: &str) -> bool {
        let lower = url.to_lowercase();
        lower.ends_with(".deb")
            || lower.ends_with(".flatpak")
            || lower.ends_with(".flatpakref")
            || lower.ends_with(".appimage")
            || lower.ends_with(".snap")
    }
}

impl Default for ZenEmbedView {
    fn default() -> Self {
        Self::new()
    }
}
