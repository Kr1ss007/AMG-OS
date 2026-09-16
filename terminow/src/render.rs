//! Terminow GPU Rendering Engine
//!
//! Owns WGPU instance, surface, device, queue, and cosmic-text text shaping
//! with JetBrains Mono font rendering on pure Wayland surfaces.
//! Directly rasterizes shaped glyphs into the surface texture buffer for 144Hz display.

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent};
use std::sync::Arc;
use winit::window::Window;

pub struct GpuRenderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub text_buffer: Buffer,
    pub font_size_pt: f32,
    pub pixel_buffer: Vec<u8>,
}

impl GpuRenderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find a compatible GPU adapter for Terminow");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Terminow Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await?;

        let mut config = surface
            .get_default_config(&adapter, width, height)
            .expect("Surface unsupported by GPU adapter");
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST;
        config.present_mode = wgpu::PresentMode::Fifo; // VSync aligned
        surface.configure(&device, &config);

        let font_size_pt = 13.0;
        let line_height_pt = font_size_pt * 1.35;
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let metrics = Metrics::new(font_size_pt, line_height_pt);
        let text_buffer = Buffer::new(&mut font_system, metrics);
        let pixel_buffer = vec![0u8; (width * height * 4) as usize];

        Ok(Self {
            surface,
            device,
            queue,
            config,
            font_system,
            swash_cache,
            text_buffer,
            font_size_pt,
            pixel_buffer,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.text_buffer
                .set_size(Some(width as f32), Some(height as f32));
            self.pixel_buffer = vec![0u8; (width * height * 4) as usize];
        }
    }

    pub fn update_grid_text(&mut self, text: &str) {
        self.text_buffer.set_text(
            text,
            &Attrs::new().family(Family::Name("JetBrains Mono")),
            Shaping::Advanced,
            None,
        );
        self.text_buffer
            .shape_until_scroll(&mut self.font_system, false);
    }

    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn render(&mut self) -> Result<(), String> {
        let width = self.config.width as usize;
        let height = self.config.height as usize;
        if width == 0 || height == 0 {
            return Ok(());
        }

        let is_bgra = matches!(
            self.config.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );

        // 1. Clear CPU pixel buffer to deep obsidian (#121214)
        let (bg_r, bg_g, bg_b, bg_a) = (18u8, 18u8, 20u8, 255u8);
        let (r_idx, b_idx) = if is_bgra { (2, 0) } else { (0, 2) };

        for chunk in self.pixel_buffer.chunks_exact_mut(4) {
            chunk[r_idx] = bg_r;
            chunk[1] = bg_g;
            chunk[b_idx] = bg_b;
            chunk[3] = bg_a;
        }

        // 2. Rasterize glyphs directly into the pixel buffer
        let (text_r, text_g, text_b) = (224u8, 224u8, 224u8);

        for run in self.text_buffer.layout_runs() {
            let line_y = run.line_y as i32;
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                if let Some(image) = self
                    .swash_cache
                    .get_image_uncached(&mut self.font_system, physical.cache_key)
                {
                    let gx = physical.x + image.placement.left;
                    let gy = line_y - image.placement.top;

                    let img_w = image.placement.width as usize;
                    let img_h = image.placement.height as usize;

                    match image.content {
                        SwashContent::Mask => {
                            for iy in 0..img_h {
                                let py = gy + iy as i32;
                                if py < 0 || py >= height as i32 {
                                    continue;
                                }
                                for ix in 0..img_w {
                                    let px = gx + ix as i32;
                                    if px < 0 || px >= width as i32 {
                                        continue;
                                    }
                                    let alpha = image.data[iy * img_w + ix];
                                    if alpha > 0 {
                                        let offset = (py as usize * width + px as usize) * 4;
                                        let a = alpha as u32;
                                        let inv_a = 255 - a;

                                        let cur_r = self.pixel_buffer[offset + r_idx] as u32;
                                        let cur_g = self.pixel_buffer[offset + 1] as u32;
                                        let cur_b = self.pixel_buffer[offset + b_idx] as u32;

                                        self.pixel_buffer[offset + r_idx] =
                                            ((text_r as u32 * a + cur_r * inv_a) / 255) as u8;
                                        self.pixel_buffer[offset + 1] =
                                            ((text_g as u32 * a + cur_g * inv_a) / 255) as u8;
                                        self.pixel_buffer[offset + b_idx] =
                                            ((text_b as u32 * a + cur_b * inv_a) / 255) as u8;
                                    }
                                }
                            }
                        }
                        SwashContent::Color => {
                            for iy in 0..img_h {
                                let py = gy + iy as i32;
                                if py < 0 || py >= height as i32 {
                                    continue;
                                }
                                for ix in 0..img_w {
                                    let px = gx + ix as i32;
                                    if px < 0 || px >= width as i32 {
                                        continue;
                                    }
                                    let src_offset = (iy * img_w + ix) * 4;
                                    let sa = image.data[src_offset + 3] as u32;
                                    if sa > 0 {
                                        let offset = (py as usize * width + px as usize) * 4;
                                        let sr = image.data[src_offset] as u32;
                                        let sg = image.data[src_offset + 1] as u32;
                                        let sb = image.data[src_offset + 2] as u32;
                                        let inv_a = 255 - sa;

                                        let cur_r = self.pixel_buffer[offset + r_idx] as u32;
                                        let cur_g = self.pixel_buffer[offset + 1] as u32;
                                        let cur_b = self.pixel_buffer[offset + b_idx] as u32;

                                        self.pixel_buffer[offset + r_idx] =
                                            ((sr * sa + cur_r * inv_a) / 255) as u8;
                                        self.pixel_buffer[offset + 1] =
                                            ((sg * sa + cur_g * inv_a) / 255) as u8;
                                        self.pixel_buffer[offset + b_idx] =
                                            ((sb * sa + cur_b * inv_a) / 255) as u8;
                                    }
                                }
                            }
                        }
                        SwashContent::SubpixelMask => {
                            for iy in 0..img_h {
                                let py = gy + iy as i32;
                                if py < 0 || py >= height as i32 {
                                    continue;
                                }
                                for ix in 0..img_w {
                                    let px = gx + ix as i32;
                                    if px < 0 || px >= width as i32 {
                                        continue;
                                    }
                                    let src_offset = (iy * img_w + ix) * 3;
                                    let mr = image.data[src_offset] as u32;
                                    let mg = image.data[src_offset + 1] as u32;
                                    let mb = image.data[src_offset + 2] as u32;

                                    let offset = (py as usize * width + px as usize) * 4;
                                    let cur_r = self.pixel_buffer[offset + r_idx] as u32;
                                    let cur_g = self.pixel_buffer[offset + 1] as u32;
                                    let cur_b = self.pixel_buffer[offset + b_idx] as u32;

                                    self.pixel_buffer[offset + r_idx] =
                                        ((text_r as u32 * mr + cur_r * (255 - mr)) / 255) as u8;
                                    self.pixel_buffer[offset + 1] =
                                        ((text_g as u32 * mg + cur_g * (255 - mg)) / 255) as u8;
                                    self.pixel_buffer[offset + b_idx] =
                                        ((text_b as u32 * mb + cur_b * (255 - mb)) / 255) as u8;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Acquire surface texture and write pixel buffer
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                self.surface.configure(&self.device, &self.config);
                tex
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            other => return Err(format!("Surface texture acquisition failure: {:?}", other)),
        };

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &surface_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.pixel_buffer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some((width * 4) as u32),
                rows_per_image: Some(height as u32),
            },
            wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
        );

        self.queue.present(surface_texture);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmic_text_font_shaping_and_metrics() {
        let mut font_system = FontSystem::new();
        let metrics = Metrics::new(13.0, 13.0 * 1.35);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        buffer.set_text(
            "amgos@workstation:~$ uname -a\nAMG-OS 0.0.1-upstream-color #1 SMP PREEMPT_DYNAMIC x86_64",
            &Attrs::new().family(Family::Name("JetBrains Mono")),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut font_system, false);

        assert_eq!(buffer.lines.len(), 2);
    }
}
