#!/usr/bin/env bash
# ==============================================================================
# AMGOS — Asset & Upstream Fork Downloader
#
# Downloads and installs:
# 1. Official Typefaces (Inter, Young Serif, Panamera, JetBrains Mono)
# 2. macOS-inspired Cursor Pack (Apple Cursor v2.0.1)
# 3. macOS-inspired Icon Pack (WhiteSur Icon Theme)
# 4. Upstream Snapshots for FORKS/ (cosmic-comp and zen-browser desktop)
# ==============================================================================

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "================================================================================"
echo "AMGOS — Downloading Typefaces, Icons, Cursors, and Upstream Forks"
echo "================================================================================"

# ------------------------------------------------------------------------------
# 1. TYPEFACES
# ------------------------------------------------------------------------------
echo "[1/4] Setting up Typefaces in assets/fonts/..."
FONTS_DIR="$REPO_ROOT/assets/fonts"
mkdir -p "$FONTS_DIR/inter" "$FONTS_DIR/young-serif" "$FONTS_DIR/panamera" "$FONTS_DIR/jetbrains-mono"

# 1.1 Inter (Main / Display)
echo "  -> Downloading Inter..."
curl -sL "https://github.com/google/fonts/raw/main/ofl/inter/Inter%5Bopsz%2Cwght%5D.ttf" \
    -o "$FONTS_DIR/inter/Inter-Variable.ttf"
curl -sL "https://github.com/google/fonts/raw/main/ofl/inter/Inter-Italic%5Bopsz%2Cwght%5D.ttf" \
    -o "$FONTS_DIR/inter/Inter-Italic-Variable.ttf"

# 1.2 Young Serif (Primary / Body)
echo "  -> Downloading Young Serif..."
curl -sL "https://github.com/google/fonts/raw/main/ofl/youngserif/YoungSerif-Regular.ttf" \
    -o "$FONTS_DIR/young-serif/YoungSerif-Regular.ttf"

# 1.3 JetBrains Mono (CLI / Monospace)
echo "  -> Downloading JetBrains Mono..."
curl -sL "https://github.com/google/fonts/raw/main/ofl/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf" \
    -o "$FONTS_DIR/jetbrains-mono/JetBrainsMono-Variable.ttf"
curl -sL "https://github.com/google/fonts/raw/main/ofl/jetbrainsmono/JetBrainsMono-Italic%5Bwght%5D.ttf" \
    -o "$FONTS_DIR/jetbrains-mono/JetBrainsMono-Italic-Variable.ttf"

# 1.4 Panamera (Secondary Labels / Metadata)
echo "  -> Downloading Panamera..."
for weight in Regular Medium Bold Light Thin Black; do
    echo "     * Panamera-$weight.otf"
    curl -sL "https://raw.githubusercontent.com/noirblancrouge/Panamera/master/fonts/otf/Panamera-$weight.otf" \
        -o "$FONTS_DIR/panamera/Panamera-$weight.otf"
done

echo "Typefaces downloaded successfully."

# ------------------------------------------------------------------------------
# 2. CURSORS (macOS Inspired)
# ------------------------------------------------------------------------------
echo "[2/4] Setting up macOS Cursors in assets/cursors/..."
CURSORS_DIR="$REPO_ROOT/assets/cursors"
mkdir -p "$CURSORS_DIR"
TMP_CURSOR_DIR="$(mktemp -d)"

echo "  -> Downloading Apple Cursor (macOS) v2.0.1..."
curl -sL "https://github.com/ful1e5/apple_cursor/releases/download/v2.0.1/macOS.tar.xz" \
    -o "$TMP_CURSOR_DIR/macOS.tar.xz"
curl -sL "https://github.com/ful1e5/apple_cursor/releases/download/v2.0.1/macOS-White.tar.xz" \
    -o "$TMP_CURSOR_DIR/macOS-White.tar.xz"

tar -xf "$TMP_CURSOR_DIR/macOS.tar.xz" -C "$CURSORS_DIR/"
tar -xf "$TMP_CURSOR_DIR/macOS-White.tar.xz" -C "$CURSORS_DIR/"
rm -rf "$TMP_CURSOR_DIR"
echo "macOS Cursor packs installed in assets/cursors/."

# ------------------------------------------------------------------------------
# 3. ICONS (macOS Inspired - WhiteSur)
# ------------------------------------------------------------------------------
echo "[3/4] Setting up macOS Icons (WhiteSur Icon Theme) in assets/icons/..."
ICONS_DIR="$REPO_ROOT/assets/icons"
mkdir -p "$ICONS_DIR"
TMP_ICON_DIR="$(mktemp -d)"

echo "  -> Downloading WhiteSur Icon Theme..."
curl -sL "https://github.com/vinceliuice/WhiteSur-icon-theme/archive/refs/heads/master.tar.gz" \
    -o "$TMP_ICON_DIR/whitesur.tar.gz"

tar -xzf "$TMP_ICON_DIR/whitesur.tar.gz" -C "$TMP_ICON_DIR/"
# Move icons into assets/icons/WhiteSur
rm -rf "$ICONS_DIR/WhiteSur"
mv "$TMP_ICON_DIR/WhiteSur-icon-theme-master" "$ICONS_DIR/WhiteSur"
rm -rf "$TMP_ICON_DIR"
echo "WhiteSur Icon theme installed in assets/icons/WhiteSur."

# ------------------------------------------------------------------------------
# 4. FORKS (COSMIC & ZEN BROWSER)
# ------------------------------------------------------------------------------
echo "[4/4] Setting up upstream snapshots in FORKS/..."
mkdir -p "$REPO_ROOT/FORKS"
TMP_FORK_DIR="$(mktemp -d)"

# 4.1 COSMIC Compositor
echo "  -> Fetching COSMIC compositor upstream snapshot..."
curl -sL "https://github.com/pop-os/cosmic-comp/archive/refs/heads/master.tar.gz" \
    -o "$TMP_FORK_DIR/cosmic.tar.gz"
rm -rf "$REPO_ROOT/FORKS/cosmic"
mkdir -p "$REPO_ROOT/FORKS/cosmic"
tar -xzf "$TMP_FORK_DIR/cosmic.tar.gz" --strip-components=1 -C "$REPO_ROOT/FORKS/cosmic"
echo "COSMIC compositor snapshot extracted to FORKS/cosmic."

# 4.2 Zen Browser Desktop
echo "  -> Fetching Zen Browser desktop upstream snapshot..."
curl -sL "https://github.com/zen-browser/desktop/archive/refs/heads/dev.tar.gz" \
    -o "$TMP_FORK_DIR/zen.tar.gz"
rm -rf "$REPO_ROOT/FORKS/zen"
mkdir -p "$REPO_ROOT/FORKS/zen"
tar -xzf "$TMP_FORK_DIR/zen.tar.gz" --strip-components=1 -C "$REPO_ROOT/FORKS/zen"
echo "Zen Browser desktop snapshot extracted to FORKS/zen."

# 4.3 Ubuntu LTS Base & Kernel
echo "  -> Setting up Ubuntu 24.04 LTS base rootfs and kernel configuration..."
mkdir -p "$REPO_ROOT/FORKS/ubuntu/base/rootfs" "$REPO_ROOT/FORKS/ubuntu/kernel"
curl -sL "https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/ubuntu-base-24.04.4-base-amd64.tar.gz" \
    -o "$TMP_FORK_DIR/ubuntu-base.tar.gz"
tar -xzf "$TMP_FORK_DIR/ubuntu-base.tar.gz" -C "$REPO_ROOT/FORKS/ubuntu/base/rootfs"
if [ -f "$REPO_ROOT/FORKS/ubuntu/base/strip.sh" ]; then
    bash "$REPO_ROOT/FORKS/ubuntu/base/strip.sh"
fi
echo "Ubuntu LTS base rootfs unpacked and stripped in FORKS/ubuntu/base/rootfs."

rm -rf "$TMP_FORK_DIR"

echo "================================================================================"
echo "All typefaces, icons, cursors, and upstream forks (Ubuntu, Zen, COSMIC) successfully downloaded."
echo "================================================================================"
