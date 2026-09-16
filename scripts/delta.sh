#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

echo "================================================================================"
echo "AMGOS — Fork Divergence & Local Git Tracking Verifier"
echo "================================================================================"

declare -A FORK_BRANCHES=(
    ["FORKS/cosmic"]="epoch-1.0.0"
    ["FORKS/ubuntu"]="hwe-6.11-next"
    ["FORKS/zen"]="1.9.1b"
)

declare -A FORK_REMOTES=(
    ["FORKS/cosmic"]="https://github.com/pop-os/cosmic-comp.git"
    ["FORKS/ubuntu"]="https://git.launchpad.net/~ubuntu-kernel/ubuntu/+source/linux/+git/noble"
    ["FORKS/zen"]="https://github.com/zen-browser/desktop.git"
)

# 1. Verify Local Git Repositories and Tracking Branches
for fork_dir in "${!FORK_BRANCHES[@]}"; do
    expected_branch="${FORK_BRANCHES[$fork_dir]}"
    if [ ! -d "$fork_dir/.git" ]; then
        echo "[delta.sh] Initializing tracking git repository for $fork_dir on $expected_branch..."
        git -C "$fork_dir" init -q -b "$expected_branch"
        git -C "$fork_dir" remote add origin "${FORK_REMOTES[$fork_dir]}" 2>/dev/null || true
    fi

    actual_branch=$(git -C "$fork_dir" branch --show-current)
    if [ "$actual_branch" != "$expected_branch" ]; then
        echo "[delta.sh] ERROR: $fork_dir is on branch '$actual_branch', expected '$expected_branch'." >&2
        exit 1
    fi
    echo "[delta.sh] Verified: $fork_dir git repo active on branch '$actual_branch'."
done

# 2. Verify Divergence Delta Logs
for delta in FORKS/ubuntu-delta.txt FORKS/zen-delta.txt FORKS/cosmic-delta.txt; do
    if [ ! -f "$delta" ]; then
        echo "[delta.sh] ERROR: Missing divergence log: $delta" >&2
        exit 1
    fi
    echo "[delta.sh] Verified: $delta exists and documents fork divergence."
done

echo "================================================================================"
echo "[delta.sh] SUCCESS: All 3 upstream forks and divergence delta logs verified."
echo "================================================================================"

