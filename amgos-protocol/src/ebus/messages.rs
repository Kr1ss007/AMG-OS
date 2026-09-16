//! AMGOS EventBus (e-bus) Typed Message Contracts
//!
//! Re-exports typed contracts from:
//! - `crate::system` (Messages published by Process 1)
//! - `crate::desktop` (Messages dispatched by Process 2)

pub use crate::desktop::DesktopRequest;
pub use crate::system::{
    AbSlotStatus, AstrophageLevel, AstrophageRecord, BluetoothDeviceInfo, DownloadJobStatus,
    InputDeviceInfo, InputDeviceType, InspectionReport, InstallStage, KeyboardConfig,
    PathfinderCategory, PathfinderItem, PowerProfile, PowerState, ProtocolFileEntry, SystemEvent,
    TouchpadConfig, WifiAccessPoint,
};
