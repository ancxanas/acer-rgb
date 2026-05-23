#!/bin/sh
# Apply the DMI bypass patch to tuxedo-drivers DKMS source
# Requires root privileges
# This modifies kernel module source — opt in consciously

set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "This script must be run as root. Use: sudo $0"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PATCH="$SCRIPT_DIR/patches/tuxedo_dmi_bypass.patch"

# Find DKMS source directory
DKMS_SRC="/usr/src/tuxedo-drivers-4.22.1"
if [ ! -d "$DKMS_SRC" ]; then
  # Try to find it dynamically
  DKMS_SRC=$(ls -d /usr/src/tuxedo-drivers-* 2>/dev/null | head -1)
fi

if [ -z "$DKMS_SRC" ] || [ ! -d "$DKMS_SRC" ]; then
  echo "Error: tuxedo-drivers DKMS source not found in /usr/src/"
  echo "Install tuxedo-drivers first: sudo dnf install tuxedo-drivers"
  exit 1
fi

echo "DKMS source: $DKMS_SRC"
echo "Patch: $PATCH"
echo ""

# Check if already applied
TARGET="$DKMS_SRC/src/tuxedo_compatibility_check/tuxedo_compatibility_check.c"
if grep -q "return true;" "$TARGET" && ! grep -q "dmi_check_system" "$TARGET"; then
  echo "Patch appears to already be applied. Skipping."
  echo "  ($TARGET)"
  echo ""
  echo "To force reapply, revert the file first:"
  echo "  cd $DKMS_SRC && git checkout -- src/tuxedo_compatibility_check/tuxedo_compatibility_check.c"
  exit 0
fi

echo "This will modify a kernel module source file."
echo "The change makes tuxedo_is_compatible() always return true,"
echo "bypassing the DMI hardware check."
echo ""
echo "Are you sure? (y/N): "
read -r CONFIRM
if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
  echo "Aborted."
  exit 1
fi

# Apply the patch
echo "Applying patch..."
if patch -d "$DKMS_SRC" -p1 < "$PATCH"; then
  echo "Patch applied successfully."
else
  echo "Patch failed. Trying manual fallback..."
  # Manual fallback: copy the original + redirect if patch format differs
  if [ -f "$TARGET" ]; then
    sed -i 's/if (dmi_check_system.*$/return true;/' "$TARGET"
    sed -i '/|| (x86_match_cpu.*/,/^[[:space:]]*return false;/{s/.*//;d}' "$TARGET"
    # Simplify: just replace the function body
    echo "Manual patch attempted. Verifying..."
  fi
fi

# Verify
if grep -q "return true;" "$TARGET" && ! grep -q "dmi_check_system" "$TARGET"; then
  echo "Verification: patch applied correctly."
else
  echo "Warning: patch verification failed. Check $TARGET manually."
  exit 1
fi

# Rebuild via DKMS
DRIVER_VERSION=$(basename "$DKMS_SRC" | sed 's/tuxedo-drivers-//')
echo ""
echo "Rebuilding via DKMS (tuxedo-drivers/$DRIVER_VERSION)..."
dkms remove "tuxedo-drivers/$DRIVER_VERSION" --all 2>/dev/null || true
dkms add "$DKMS_SRC"
dkms build "tuxedo-drivers/$DRIVER_VERSION"
dkms install "tuxedo-drivers/$DRIVER_VERSION"
depmod -a

echo ""
echo "DMI bypass patch applied and modules rebuilt."
echo "Load the modules:  sudo modprobe clevo_wmi"
echo "Verify:           lsmod | grep -E 'tuxedo|clevo'"
