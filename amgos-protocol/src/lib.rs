//! amgos-protocol: Shared IPC Contracts and Transport Types
//!
//! Provides the binary pub/sub EventBus (e-bus) for Process 1 <-> Process 2 communication
//! and EventOutsiderBus (eo-bus) contracts for external application bridging.
//!
//! Conforms to AMGOS_GIT_STRUCTURE_v0.0.1.txt:
//! - `system/`: Messages published by Process 1
//! - `desktop/`: Messages published by Process 2
//! - `ebus/`: Internal framed pub/sub transport over Unix domain sockets
//! - `eobus/`: External application D-Bus bridge contracts

pub mod desktop;
pub mod ebus;
pub mod eobus;
pub mod system;

pub use desktop::*;
pub use ebus::*;
pub use eobus::*;
pub use system::*;
