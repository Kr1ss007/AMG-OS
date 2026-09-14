//! amgos-protocol: Shared IPC Contracts and Transport Types
//!
//! Provides the binary pub/sub EventBus (e-bus) for Process 1 <-> Process 2 communication
//! and EventOutsiderBus (eo-bus) contracts for external application bridging.

pub mod ebus;
pub mod eobus;

pub use ebus::*;
pub use eobus::*;
