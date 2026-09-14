amgos-protocol
================================================================================
Shared binary IPC contracts and transport protocol for AMGOS.

Modules:
- ebus/ (EventBus / e-bus):
  Custom framed binary pub/sub protocol over Unix domain sockets.
  Process 1 publishes SystemEvent; Process 2 subscribes and sends DesktopRequest.
- eobus/ (EventOutsiderBus / eo-bus):
  Contracts for external third-party applications via D-Bus.
  Strictly isolated from internal pub/sub.
================================================================================
