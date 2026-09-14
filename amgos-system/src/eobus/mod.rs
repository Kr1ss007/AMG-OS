//! EventOutsiderBus (eo-bus) Subsystem in Process 1
//!
//! Exposes sanitized external interfaces over D-Bus to third-party applications.
//! Completely isolated from internal EventBus.

pub use amgos_protocol::eobus::*;
