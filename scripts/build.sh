#!/usr/bin/env bash
set -euo pipefail

echo "================================================================================"
echo "AMGOS — Workspace Build Pipeline"
echo "================================================================================"

cargo build --workspace --release
echo "[build.sh] Full workspace build complete."
