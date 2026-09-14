#!/usr/bin/env bash
set -euo pipefail

echo "================================================================================"
echo "AMGOS — Fork Divergence Delta Verifier"
echo "================================================================================"

for delta in FORKS/zen-delta.txt FORKS/cosmic-delta.txt; do
    if [ ! -f "$delta" ]; then
        echo "[delta.sh] ERROR: Missing divergence log: $delta"
        exit 1
    fi
    echo "[delta.sh] Verified: $delta exists."
done
echo "[delta.sh] Fork deltas verified."
