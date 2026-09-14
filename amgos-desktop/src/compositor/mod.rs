//! Wayland Compositor Bridge
//!
//! Smithay-based compositor integration layer.
//! Wayland exclusively. Pure Wayland surfaces; no X11 or XWayland.

pub struct CompositorBridge {
    pub display_width: u32,
    pub display_height: u32,
    pub refresh_rate_hz: u32,
    pub scale_factor: f32,
}

impl CompositorBridge {
    pub fn new() -> Self {
        Self {
            display_width: 1920,
            display_height: 1080,
            refresh_rate_hz: 144, // 144Hz Full HD+ reference spec
            scale_factor: 1.0,
        }
    }

    pub fn set_geometry(&mut self, width: u32, height: u32, refresh_rate: u32, scale: f32) {
        self.display_width = width;
        self.display_height = height;
        self.refresh_rate_hz = refresh_rate;
        self.scale_factor = scale;
    }
}

impl Default for CompositorBridge {
    fn default() -> Self {
        Self::new()
    }
}
