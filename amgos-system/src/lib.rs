//! AMGOS Process 1 — System Session Library
//!
//! Exposes internal subsystems of Process 1 for supervisor, daemon, and integration testing:
//! - Hardware detection & driver monitoring
//! - Network credentials vault & connection management
//! - Power management (sleep, wake, hibernate, shutdown) with profiles (Endurance, Balanced, MAX)
//! - EventBus (e-bus) internal pub/sub broker
//! - Audio-Video Manager (AVM) with PipeWire/WirePlumber & boot chime sequencer
//! - Astrophage continuous rolling diagnostic logger
//! - Security boundaries & signed permission state
//! - Service supervisor

pub mod astrophage;
pub mod avm;
pub mod ebus;
pub mod eobus;
pub mod hardware;
pub mod motionwave;
pub mod network;
pub mod permissions;
pub mod power;
pub mod supervisor;

pub use astrophage::AstrophageCoreLogger;
pub use avm::{AudioVideoManager, ChimeEvent, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE, CHIME_DURATION_MS, CHIME_FREQUENCY_HZ, CHIME_NOTE};
pub use ebus::{EventBusServer, DEFAULT_EBUS_SOCKET_PATH};
pub use eobus::EventOutsiderBusBridge;
pub use hardware::{detect_hardware, HardwareProfile};
pub use motionwave::MotionWaveController;
pub use network::{NetworkCredentialVault, NetworkStatus};
pub use permissions::PermissionManager;
pub use power::PowerManager;
pub use supervisor::ServiceSupervisor;
