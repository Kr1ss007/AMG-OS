//! Filer Shell: Process 2 Frontend UI for Filer & Pathfinder
//!
//! Owns Pathfinder search bar UI, Zen Browser embed container,
//! dual-pane file browser, and symmetric install/uninstall dialogs.
//!
//! Must run as a Wayland surface.

use amgos_protocol::ebus::{DesktopRequest, EventBusClient, DEFAULT_EBUS_SOCKET_PATH};
use crossbeam_channel::unbounded;
use filer_shell::{FileBrowserPanel, PackageActionDialog, PathfinderSearchBar, ZenEmbedView};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct FilerApp {
    window: Option<Arc<Window>>,
    pathfinder: PathfinderSearchBar,
    browser: Option<FileBrowserPanel>,
    #[allow(dead_code)]
    zen_embed: Option<ZenEmbedView>,
    #[allow(dead_code)]
    active_dialog: Option<PackageActionDialog>,
}

impl ApplicationHandler for FilerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        winit::window::Window::default_attributes().with_title("Filer - AMG-OS"),
                    )
                    .unwrap(),
            );
            self.window = Some(window.clone());

            self.pathfinder = PathfinderSearchBar::new();
            self.browser = Some(FileBrowserPanel::new("/home"));
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
                // Filer never quits. In a real environment, it would just hide itself here.
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Zero TTY output in production

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = Arc::clone(&running);
    let (_event_tx, _event_rx) = unbounded();

    // EventBus background thread for communicating with Process 1 (Filer Core)
    if let Ok(client) = EventBusClient::connect(DEFAULT_EBUS_SOCKET_PATH) {
        // Query Pathfinder immediately
        let req = DesktopRequest::SearchPathfinder {
            query_id: 1,
            query: "welcome".to_string(),
            max_results: 10,
        };
        let _ = client.send_request(&req);

        thread::spawn(move || {
            while r_clone.load(Ordering::SeqCst) {
                if let Ok(event) = client.read_event() {
                    let _ = _event_tx.send(event);
                } else {
                    thread::sleep(Duration::from_millis(50));
                }
            }
        });
    }

    // A background thread would process `event_rx` here and update shared state
    // consumed by the Winit redraw loop.

    let event_loop = EventLoop::new().unwrap();
    let mut app = FilerApp::default();

    // Run the Wayland native Winit event loop
    event_loop.run_app(&mut app)?;

    Ok(())
}
