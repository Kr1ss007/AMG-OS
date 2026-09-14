astrophage — Hardware and System Diagnostic Application
================================================================================
Astrophage keeps the AMGOS hardware compatibility guarantee accurate over time.
- core/: Process 1 rolling ring buffer (10,000 entries) capturing thermals,
  memory status, NVMe health, and subsystem events.
- shell/: Process 2 user-facing report builder with privacy scrubbing.
- protocol/: Shared issue format and telemetry contracts.
================================================================================
