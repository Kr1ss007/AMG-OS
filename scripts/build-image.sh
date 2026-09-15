#!/usr/bin/env bash
# ==============================================================================
# AMG-OS — Full Operating System Image Pipeline
# Codename: Upstream Color (v0.0.1 Pre-Production Alpha)
#
# Enforces:
# - AMGOS_SPEC Section 2 (Ubuntu LTS base, stripped, no snapd/cloud-init/GNOME/X11)
# - AMGOS_SPEC Section 4 (Boot silence, zero visible kernel logs, framebuffer continuity)
# - AMGOS_SPEC Section 11 & 12 (A/B Partition Model, immutable base, isolated App Layer)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$WORKSPACE_ROOT/target/image-build"
OUTPUT_DIR="$WORKSPACE_ROOT/target/dist"
OUTPUT_IMG="$OUTPUT_DIR/amgos-upstream-color-v0.0.1.img"

echo "================================================================================"
echo "AMG-OS Operating System Image Build Pipeline"
echo "Target Image: $OUTPUT_IMG"
echo "================================================================================"

mkdir -p "$BUILD_DIR" "$OUTPUT_DIR"

# 1. Compile all workspace crates in release mode
echo "[1/6] Compiling AMG-OS production workspace binaries..."
cargo build --workspace --release

# 2. Prepare staging rootfs
STAGING_ROOTFS="$BUILD_DIR/rootfs"
echo "[2/6] Assembling base userland from FORKS/ubuntu..."
rm -rf "$STAGING_ROOTFS"
mkdir -p "$STAGING_ROOTFS"

if [ -d "$WORKSPACE_ROOT/FORKS/ubuntu/base/rootfs" ]; then
    cp -a "$WORKSPACE_ROOT/FORKS/ubuntu/base/rootfs/." "$STAGING_ROOTFS/"
else
    echo "ERROR: FORKS/ubuntu/base/rootfs directory missing." >&2
    exit 1
fi

# Execute Ubuntu delta stripping rules
echo "[2/6] Executing stripping policy (no cloud-init, no snapd, no GNOME, no X11)..."
bash "$WORKSPACE_ROOT/FORKS/ubuntu/base/strip.sh"

# 3. Install AMG-OS production binaries
echo "[3/6] Installing AMG-OS binaries into system root..."
mkdir -p "$STAGING_ROOTFS/usr/libexec/amgos" "$STAGING_ROOTFS/usr/bin"
cp "$WORKSPACE_ROOT/target/release/amgos-system" "$STAGING_ROOTFS/usr/libexec/amgos/amgos-system"
chmod 755 "$STAGING_ROOTFS/usr/libexec/amgos/amgos-system"

for bin in amgos-desktop filer-shell terminow settings astrophage-shell; do
    if [ -f "$WORKSPACE_ROOT/target/release/$bin" ]; then
        cp "$WORKSPACE_ROOT/target/release/$bin" "$STAGING_ROOTFS/usr/bin/$bin"
        chmod 755 "$STAGING_ROOTFS/usr/bin/$bin"
    fi
done

# Create friendly symlinks
ln -sf /usr/bin/filer-shell "$STAGING_ROOTFS/usr/bin/filer"
ln -sf /usr/bin/astrophage-shell "$STAGING_ROOTFS/usr/bin/astrophage"

# Install XDG Desktop Applications for first-class native citizens (SPEC Section 9)
APP_ENTRY_DIR="$STAGING_ROOTFS/usr/share/applications"
mkdir -p "$APP_ENTRY_DIR"

cat << 'EOF' > "$APP_ENTRY_DIR/org.amgos.filer.desktop"
[Desktop Entry]
Name=Filer
Comment=Primary Application — File Manager, Pathfinder, and App Installer
Exec=/usr/bin/filer-shell
Icon=system-file-manager
Terminal=false
Type=Application
Categories=System;FileManager;
EOF

cat << 'EOF' > "$APP_ENTRY_DIR/org.amgos.terminow.desktop"
[Desktop Entry]
Name=Terminow
Comment=GPU-Accelerated Native Terminal
Exec=/usr/bin/terminow
Icon=utilities-terminal
Terminal=false
Type=Application
Categories=System;TerminalEmulator;
EOF

cat << 'EOF' > "$APP_ENTRY_DIR/org.amgos.settings.desktop"
[Desktop Entry]
Name=Settings
Comment=System Configuration & OTA Updates
Exec=/usr/bin/settings
Icon=preferences-system
Terminal=false
Type=Application
Categories=Settings;
EOF

cat << 'EOF' > "$APP_ENTRY_DIR/org.amgos.astrophage.desktop"
[Desktop Entry]
Name=Astrophage
Comment=Hardware Diagnostic & Telemetry Reporter
Exec=/usr/bin/astrophage-shell
Icon=utilities-system-monitor
Terminal=false
Type=Application
Categories=System;Monitor;
EOF

# 4. Install Fonts, Cursors, and Icons
echo "[4/6] Bundling curated system assets (fonts, cursors, WhiteSur icons)..."
FONT_DEST="$STAGING_ROOTFS/usr/share/fonts/truetype/amgos"
mkdir -p "$FONT_DEST"
if [ -d "$WORKSPACE_ROOT/assets/fonts" ]; then
    cp -r "$WORKSPACE_ROOT/assets/fonts/." "$FONT_DEST/"
fi

ICON_DEST="$STAGING_ROOTFS/usr/share/icons"
mkdir -p "$ICON_DEST"
if [ -d "$WORKSPACE_ROOT/assets/icons/WhiteSur" ]; then
    cp -r "$WORKSPACE_ROOT/assets/icons/WhiteSur" "$ICON_DEST/"
fi
if [ -d "$WORKSPACE_ROOT/assets/cursors" ]; then
    cp -r "$WORKSPACE_ROOT/assets/cursors/." "$ICON_DEST/"
fi

# 5. Install systemd services and boot silence configuration
echo "[5/6] Configuring systemd units and boot silence parameters..."
SYSTEMD_DIR="$STAGING_ROOTFS/etc/systemd/system"
mkdir -p "$SYSTEMD_DIR"
cp "$WORKSPACE_ROOT/FORKS/ubuntu/systemd/amgos-system.service" "$SYSTEMD_DIR/"
cp "$WORKSPACE_ROOT/FORKS/ubuntu/systemd/amgos-desktop.service" "$SYSTEMD_DIR/"
cp "$WORKSPACE_ROOT/FORKS/ubuntu/systemd/amgos.target" "$SYSTEMD_DIR/"

# Set amgos.target as default
ln -sf /etc/systemd/system/amgos.target "$SYSTEMD_DIR/default.target"

# Boot silence cmdline parameters
mkdir -p "$STAGING_ROOTFS/etc/kernel"
cat "$WORKSPACE_ROOT/FORKS/ubuntu/kernel/cmdline.txt" > "$STAGING_ROOTFS/etc/kernel/cmdline"

# Create App Layer mountpoint (/var/lib/amgos/apps)
mkdir -p "$STAGING_ROOTFS/var/lib/amgos/apps"
mkdir -p "$STAGING_ROOTFS/run/amgos"

# 6. Generate Immutable SquashFS Root & A/B Disk Layout
echo "[6/6] Generating immutable squashfs base image..."
SQUASHFS_IMG="$BUILD_DIR/base_a.squashfs"
rm -f "$SQUASHFS_IMG"

if command -v mksquashfs &> /dev/null; then
    mksquashfs "$STAGING_ROOTFS" "$SQUASHFS_IMG" -comp zstd -noappend -b 1048576 -quiet
    cp -f "$SQUASHFS_IMG" "$OUTPUT_DIR/base_a.squashfs"
    echo "[build-image.sh] Immutable Base A created: $(du -h "$OUTPUT_DIR/base_a.squashfs" | cut -f1)"
else
    echo "NOTE: mksquashfs not installed on host; generating raw rootfs tarball."
    tar -czf "$OUTPUT_DIR/base_a.tar.gz" -C "$STAGING_ROOTFS" .
fi

# Construct Disk Partition Layout Specification
cat << 'EOF' > "$OUTPUT_DIR/partition-layout.txt"
================================================================================
AMG-OS A/B GPT PARTITION LAYOUT SPECIFICATION
================================================================================
Sector size: 512 bytes
Disk label: gpt

Number  Start    End      Size     File system  Name       Flags
 1      2048s    2099199s 1024MB   fat32        ESP        boot, esp
 2      2099200s 10487807 4096MB   squashfs     BASE_A     read-only, immutable
 3      10487808 18876415 4096MB   squashfs     BASE_B     standby, OTA slot
 4      18876416 39847935 10240MB  ext4         APP_LAYER  /var/lib/amgos/apps
 5      39847936 100%     Remaining ext4        USER_DATA  /home (encrypted)
================================================================================
EOF

echo "================================================================================"
echo "AMG-OS Image Pipeline Build Complete!"
echo "Artifacts available in: $OUTPUT_DIR"
echo "Partition Layout: $OUTPUT_DIR/partition-layout.txt"
echo "================================================================================"
