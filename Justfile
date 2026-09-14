# AMGOS Root Justfile
# Unified build, test, and virtualization driver

export PATH := env_var('HOME') + "/.cargo/bin:" + env_var('PATH')

# Default recipe: check entire workspace
default: check

# Check all crates across the workspace for compile correctness
check:
    cargo check --workspace --all-targets

# Build release artifacts for the entire workspace
build:
    cargo build --workspace --release

# Run automated tests across all workspace crates
test:
    cargo test --workspace

# Format all workspace code
fmt:
    cargo fmt --all

# Lint the workspace with clippy
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run Process 1 System Session locally
run-system:
    cargo run -p amgos-system

# Run Process 2 Desktop Session locally
run-desktop:
    cargo run -p amgos-desktop

# Launch test VM in QEMU with KVM and OVMF UEFI
run-qemu *ARGS:
    bash scripts/run-qemu.sh {{ARGS}}

# Clean build artifacts
clean:
    cargo clean
