FORKS/ubuntu — Ubuntu LTS Base & Kernel Fork
================================================================================
Upstream: Ubuntu 24.04 LTS (Noble Numbat)
Role: Infrastructure floor (Kernel, hardware detection, firmware infrastructure).

Directories:
- base/: Stripped Ubuntu LTS base userland rootfs and stripping manifests.
- kernel/: Canonical HWE kernel configuration, parameters, and silence flags.

Governing Policy:
- C inside the kernel (untouched upstream).
- Rust exclusively above the kernel.
- No Ubuntu branding, no GNOME, no snapd, no cloud-init, no X11/XWayland.
================================================================================
