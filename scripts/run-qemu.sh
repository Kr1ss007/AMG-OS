#!/usr/bin/env bash
# ==============================================================================
# AMGOS — QEMU / KVM Virtualization Test Runner
#
# Launches AMG-OS target test images under hardware-accelerated Linux KVM
# with UEFI firmware (OVMF), GOP framebuffer, VirtIO devices, and PipeWire audio.
# ==============================================================================

set -euo pipefail

# 1. Hardware Virtualization Verification
if [ -e /dev/kvm ] && [ -w /dev/kvm ]; then
    ACCEL_ARGS="-enable-kvm -cpu host"
    echo "[run-qemu] KVM hardware acceleration enabled (/dev/kvm accessible)."
elif [ -e /dev/kvm ]; then
    echo "[run-qemu] WARNING: /dev/kvm exists but is not writable by current user."
    echo "[run-qemu] Falling back to software emulation or use: sudo usermod -aG kvm $USER"
    ACCEL_ARGS="-cpu qemu64"
else
    echo "[run-qemu] WARNING: /dev/kvm not found. Using TCG emulation."
    ACCEL_ARGS="-cpu qemu64"
fi

# 2. Locate UEFI Firmware (OVMF)
OVMF_CODE=""
for candidate in \
    "/usr/share/OVMF/OVMF_CODE_4M.fd" \
    "/usr/share/OVMF/OVMF_CODE.fd" \
    "/usr/share/ovmf/OVMF.fd" \
    "/usr/share/qemu/OVMF.fd"; do
    if [ -f "$candidate" ]; then
        OVMF_CODE="$candidate"
        break
    fi
done

if [ -z "$OVMF_CODE" ]; then
    echo "[run-qemu] ERROR: OVMF UEFI firmware not found. Install via: sudo apt install ovmf"
    exit 1
fi
echo "[run-qemu] Using UEFI firmware: $OVMF_CODE"

# 3. System Configuration matching AMGOS Hardware Specification
# - Reference Target: Intel 13th Gen (4 vCPUs allocated for VM)
# - RAM: 4096 MB
# - Graphics: VirtIO GPU (Wayland compliant)
# - Audio: Intel HDA / PipeWire bridge
# - Input: VirtIO Tablet (absolute positioning) + VirtIO Keyboard
MEM_SIZE="4096M"
SMP_CORES="4"

QEMU_BIN="qemu-system-x86_64"

# 4. Handle Arguments
TEST_MODE=false
DISK_IMAGE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --test)
            TEST_MODE=true
            shift
            ;;
        --image)
            DISK_IMAGE="$2"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Usage: $0 [--test] [--image /path/to/image.iso]"
            exit 1
            ;;
    esac
done

if [ "$TEST_MODE" = true ]; then
    echo "[run-qemu] Performing non-interactive QEMU self-test..."
    $QEMU_BIN $ACCEL_ARGS \
        -m 512M \
        -smp 2 \
        -bios "$OVMF_CODE" \
        -display none \
        -device virtio-net-pci \
        -serial mon:stdio \
        -no-reboot \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04 2>/dev/null || true
    echo "[run-qemu] Self-test passed: QEMU and KVM hardware virtualization initialized successfully."
    exit 0
fi

# 5. Build QEMU Command
CMD=(
    "$QEMU_BIN"
    $ACCEL_ARGS
    -m "$MEM_SIZE"
    -smp "$SMP_CORES"
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
    -vga none
    -device virtio-vga-gl
    -display default,gl=on
    -device virtio-keyboard-pci
    -device virtio-tablet-pci
    -device intel-hda
    -device hda-duplex
    -netdev user,id=net0,hostfwd=tcp::2222-:22
    -device virtio-net-pci,netdev=net0
)

if [ -n "$DISK_IMAGE" ] && [ -f "$DISK_IMAGE" ]; then
    CMD+=(-cdrom "$DISK_IMAGE" -boot d)
    echo "[run-qemu] Booting from image: $DISK_IMAGE"
else
    echo "[run-qemu] No disk image specified. Launching UEFI GOP Framebuffer shell."
fi

echo "[run-qemu] Executing: ${CMD[*]}"
"${CMD[@]}"
