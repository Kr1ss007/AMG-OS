#!/usr/bin/env bash
# ==============================================================================
# AMGOS — Ubuntu Base Userland Stripper
# Enforces AMGOS SPEC SECTION 2.2
# ==============================================================================

set -euo pipefail

ROOTFS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/rootfs" && pwd)"

echo "[strip.sh] Stripping Ubuntu Base in $ROOTFS_DIR..."

# 1. Remove cloud-init and server files
rm -rf "$ROOTFS_DIR/etc/cloud" "$ROOTFS_DIR/var/lib/cloud"

# 2. Remove default Ubuntu branding and motd
rm -f "$ROOTFS_DIR/etc/motd" "$ROOTFS_DIR/etc/issue" "$ROOTFS_DIR/etc/issue.net"
rm -rf "$ROOTFS_DIR/etc/update-motd.d"

# 3. Restrict /usr/bin/apt from user PATH
# apt is kept only for internal backend use in Process 1 Filer, never exposed to user
if [ -f "$ROOTFS_DIR/usr/bin/apt" ]; then
    chmod 700 "$ROOTFS_DIR/usr/bin/apt"
fi

# 4. Remove any X11 remnants
rm -rf "$ROOTFS_DIR/etc/X11"

echo "[strip.sh] Base userland successfully stripped according to AMGOS Specification."
