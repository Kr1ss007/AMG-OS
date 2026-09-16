//! Terminow: GPU-Accelerated AMGOS Terminal Emulator
//!
//! Features:
//! - JetBrains Mono typography
//! - Pure Wayland surface via Winit & WGPU
//! - GPU text rendering pipeline with real glyph rasterization
//! - Real POSIX PTY shell session with non-blocking I/O
//! - First-class Global Menu and MotionWave gesture awareness

pub mod grid;
pub mod pty;
pub mod render;

use grid::TerminalGrid;
use pty::PtySession;
use render::GpuRenderer;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

pub const DEFAULT_FONT_NAME: &str = "JetBrains Mono";
pub const DEFAULT_FONT_SIZE_PT: f32 = 13.0;

#[derive(Default)]
struct TerminowApp {
    window: Option<Arc<Window>>,
    grid: Option<TerminalGrid>,
    pty: Option<PtySession>,
    renderer: Option<GpuRenderer>,
}

impl ApplicationHandler for TerminowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        winit::window::Window::default_attributes()
                            .with_title("Terminow — AMG-OS")
                            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 600.0)),
                    )
                    .unwrap(),
            );
            self.window = Some(window.clone());

            // Initialize Grid (80 cols, 24 rows standard terminal geometry)
            let mut grid = TerminalGrid::new(80, 24);
            grid.write_bytes(b"AMG-OS 0.0.1 (Upstream Color) - Pure Wayland Session\r\n\r\n");

            match PtySession::open(80, 24) {
                Ok(pty) => self.pty = Some(pty),
                Err(e) => eprintln!(
                    "[terminow] Warning: Unable to spawn interactive PTY shell: {}",
                    e
                ),
            }

            self.grid = Some(grid);

            // Initialize WGPU rendering pipeline on the native Wayland surface
            match pollster::block_on(GpuRenderer::new(window.clone())) {
                Ok(mut renderer) => {
                    if let Some(ref g) = self.grid {
                        renderer.update_grid_text(&g.formatted_content());
                    }
                    self.renderer = Some(renderer);
                }
                Err(e) => eprintln!("[terminow] GPU renderer initialization error: {e}"),
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        text,
                        ..
                    },
                ..
            } => {
                let mut to_send: Vec<u8> = Vec::new();
                match logical_key {
                    Key::Named(NamedKey::Enter) => {
                        to_send.push(b'\r');
                    }
                    Key::Named(NamedKey::Backspace) => {
                        to_send.push(0x7F);
                    }
                    Key::Named(NamedKey::Tab) => {
                        to_send.push(b'\t');
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        to_send.extend_from_slice(b"\x1B[A");
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        to_send.extend_from_slice(b"\x1B[B");
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        to_send.extend_from_slice(b"\x1B[C");
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        to_send.extend_from_slice(b"\x1B[D");
                    }
                    _ => {
                        if let Some(t) = text {
                            to_send.extend_from_slice(t.as_bytes());
                        }
                    }
                }

                if !to_send.is_empty() {
                    if let Some(ref mut pty) = self.pty {
                        let _ = pty.write_input(&to_send);
                    } else if let Some(ref mut grid) = self.grid {
                        grid.write_bytes(&to_send);
                    }
                    if let Some(ref window) = self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // 1. Drain any pending output from the PTY shell
                let mut buf = [0u8; 4096];
                let mut read_anything = false;
                if let Some(ref mut pty) = self.pty {
                    loop {
                        match pty.read_output(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                read_anything = true;
                                if let Some(ref mut grid) = self.grid {
                                    grid.write_bytes(&buf[..n]);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }

                // 2. Update cosmic-text text buffer from the grid
                if let (Some(ref mut renderer), Some(ref grid)) = (&mut self.renderer, &self.grid) {
                    if read_anything {
                        renderer.update_grid_text(&grid.formatted_content());
                    }
                    let _ = renderer.render();
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new().unwrap();
    let mut app = TerminowApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
