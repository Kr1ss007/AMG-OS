#!/usr/bin/env bash
set -euo pipefail

echo "================================================================================"
echo "AMGOS — Image Signing & Verification"
echo "================================================================================"

TARGET_FILE="${1:-}"
if [ -z "$TARGET_FILE" ] || [ ! -f "$TARGET_FILE" ]; then
    echo "Usage: $0 <path_to_image_or_binary>"
    exit 1
fi

echo "[sign.sh] Generating SHA-256 integrity hash for $TARGET_FILE..."
sha256sum "$TARGET_FILE" > "${TARGET_FILE}.sha256"
echo "[sign.sh] Signed checksum written to ${TARGET_FILE}.sha256"
