//! AMGOS EventBus (e-bus) Module
//!
//! Owns the internal pub/sub binary protocol between Process 1 and Process 2.

pub mod frame;
pub mod messages;
pub mod transport;

pub use frame::{
    compute_checksum, encode_frame, read_frame_header, read_frame_payload, write_frame, FrameError,
    FrameHeader, EBUS_MAGIC, EBUS_PROTOCOL_VERSION,
};
pub use messages::{
    AbSlotStatus, AstrophageLevel, AstrophageRecord, BluetoothDeviceInfo, DesktopRequest,
    DownloadJobStatus, InputDeviceInfo, InputDeviceType, InspectionReport, InstallStage,
    KeyboardConfig, PathfinderCategory, PathfinderItem, PowerProfile, PowerState,
    ProtocolFileEntry, SystemEvent, TouchpadConfig, WifiAccessPoint,
};
pub use transport::{EventBusClient, EventBusServer, DEFAULT_EBUS_SOCKET_PATH};
