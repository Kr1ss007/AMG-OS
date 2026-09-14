================================================================================
AMGOS — A MULTI-PURPOSE GENERAL OPERATING SYSTEM
Codename: Upstream Color (v0.0.1 Pre-Production Alpha)
================================================================================

AMGOS is not a Linux distribution. It is not a desktop environment.
It is a complete, opinionated, stable-channel platform operating system
that uses the Linux kernel as infrastructure.

The Core Promise:
"Set it up once. It works after that. No troubleshooting."

Architecture:
- Two-Process Hard Boundary:
  * Process 1 (amgos-system): Hardware, network credentials, power, AVM, Astrophage logger.
  * Process 2 (amgos-desktop): Wayland compositor, rendered UI, cubic-bezier animation engine.
- Internal IPC: EventBus (e-bus) binary pub/sub over Unix domain sockets.
- External IPC: EventOutsiderBus (eo-bus) D-Bus bridge for third-party applications.
- Language Policy: Pure Rust above the kernel. Untouched C in Ubuntu LTS HWE kernel.
- Display: Pure Wayland exclusively. Zero X11 or XWayland.

First-Class Native Apps:
- Filer (with Pathfinder search bar) — primary always-running application.
- Zen Browser — native browser engine powered by Gecko.
- Terminow — GPU-accelerated terminal emulator with JetBrains Mono.
- Settings — central configuration surface with OTA A/B update management.
- Astrophage — system diagnostic reporter with rolling ring-buffer.

Build & Development:
- Required tool: `just` (https://github.com/casey/just)
- Commands:
  * `just check`      - Verify compile correctness across all workspace crates
  * `just build`      - Build release binaries
  * `just test`       - Run automated test suite
  * `just run-system` - Run Process 1 System Session
  * `just run-desktop`- Run Process 2 Desktop Session
  * `just run-qemu`   - Launch test VM in QEMU with KVM and UEFI (OVMF)
================================================================================
