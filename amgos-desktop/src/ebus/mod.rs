//! EventBus (e-bus) Subscriber Client in Process 2
//!
//! Connects to Process 1 over Unix domain socket to receive system events.
//! Dispatches DesktopRequest messages over e-bus without touching hardware.

pub use amgos_protocol::ebus::{
    DesktopRequest, EventBusClient, FrameError, SystemEvent, DEFAULT_EBUS_SOCKET_PATH,
};
