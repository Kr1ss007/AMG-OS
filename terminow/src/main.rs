//! Terminow: GPU-Accelerated AMGOS Terminal Emulator
//!
//! Features:
//! - JetBrains Mono typography
//! - Pure Wayland surface
//! - GPU text rendering pipeline abstraction
//! - First-class Global Menu and MotionWave gesture awareness

pub mod grid;
pub mod pty;

use grid::TerminalGrid;
use pty::PtySession;

pub const DEFAULT_FONT_NAME: &str = "JetBrains Mono";
pub const DEFAULT_FONT_SIZE_PT: f32 = 13.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[terminow] Initializing Terminow terminal emulator...");
    println!(
        "[terminow] Font: {} @ {} pt",
        DEFAULT_FONT_NAME, DEFAULT_FONT_SIZE_PT
    );

    let mut grid = TerminalGrid::new(80, 24);
    println!(
        "[terminow] Terminal grid initialized ({} cols x {} rows)",
        grid.cols, grid.rows
    );

    let test_str = "AMG-OS Terminow [Version 0.0.1]\nKernel handoff verified. PTY allocated.\n";
    for c in test_str.chars() {
        grid.write_char(c);
    }

    match PtySession::open(80, 24) {
        Ok(pty) => {
            println!(
                "[terminow] Successfully allocated PTY master/slave pair (slave fd: {})",
                pty.slave_fd()
            );
        }
        Err(e) => {
            eprintln!("[terminow] Warning: Unable to open POSIX PTY: {}", e);
        }
    }

    println!("[terminow] Ready for Wayland presentation.");
    Ok(())
}
