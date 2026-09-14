#!/usr/bin/env bash
set -euo pipefail

echo "================================================================================"
echo "AMGOS — Workspace Automated Test Pipeline"
echo "================================================================================"

cargo test --workspace
echo "[test.sh] All workspace tests passed."
