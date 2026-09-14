amgos-system
================================================================================
Process 1 — System Session (Rust)

Owns:
- Hardware, display hardware detection, audio routing
- Network credentials and connection manager
- Power management (sleep, wake, hibernate, shutdown)
- EventBus (e-bus) internal pub/sub broker
- Audio-Video Manager (AVM) — PipeWire, WirePlumber, chime sequencer
- Astrophage continuous event logger and rolling ring buffer
- Permissions and signed state management

Boundary Rule:
Process 1 never renders UI.
Process 1 never quits during normal user operation.
================================================================================
