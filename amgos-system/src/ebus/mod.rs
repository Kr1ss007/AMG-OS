//! EventBus (e-bus) Host Subsystem in Process 1
//!
//! Owns the Unix domain socket server, dispatches SystemEvent broadcasts,
//! and handles DesktopRequest messages from Process 2.

pub use amgos_protocol::ebus::{
    DesktopRequest, EventBusServer, FrameError, SystemEvent, DEFAULT_EBUS_SOCKET_PATH,
};
