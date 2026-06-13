#!/bin/sh
# Install kbd-rgbd system-wide: daemon, scripts, presets, systemd unit
# Requires root privileges

set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "This script must be run as root. Use: sudo $0"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
echo "Installing from: $SCRIPT_DIR"

# Build the Rust binary
echo "  building kbd-rgbd..."
(cd "$SCRIPT_DIR" && cargo build --release)

# Install daemon binary
echo "  /usr/local/bin/kbd-rgbd"
install -m755 "$SCRIPT_DIR"/target/release/kbd-rgbd /usr/local/bin/kbd-rgbd

# Install control scripts
echo "  /usr/local/bin/kbd-*"
for script in kbd-brightness-up kbd-brightness-down kbd-preset-switch kbd-preset-list kbd-off; do
  install -m755 "$SCRIPT_DIR/scripts/$script" "/usr/local/bin/$script"
done

# Remove old kbdctl
rm -f /usr/local/bin/kbdctl /usr/local/bin/kbd-brightness /usr/local/bin/kbd-preset

# Copy presets
echo "  presets/  -> /etc/tailord/"
install -d -m755 /etc/tailord/keyboard /etc/tailord/profiles
cp -a "$SCRIPT_DIR/presets/keyboard/"*.json /etc/tailord/keyboard/
install -m644 "$SCRIPT_DIR/presets/profiles/"*.json /etc/tailord/profiles/

# Copy modprobe configs
echo "  modprobe configs"
install -d -m755 /etc/modules-load.d /etc/modprobe.d
install -m644 "$SCRIPT_DIR/packaging/modules-load.d/clevo-wmi.conf" /etc/modules-load.d/clevo-wmi.conf
install -m644 "$SCRIPT_DIR/packaging/modprobe.d/tuxedo-keyboard.conf" /etc/modprobe.d/tuxedo-keyboard.conf

# Install systemd unit
echo "  kbd-rgbd.service"
install -m644 "$SCRIPT_DIR/packaging/kbd-rgbd.service" /etc/systemd/system/kbd-rgbd.service

# Ensure active symlink exists (default to cycle if not set)
if [ ! -L /etc/tailord/active_profile.json ]; then
  ln -sf /etc/tailord/profiles/cycle.json /etc/tailord/active_profile.json
fi

# Remove old tailord service, enable new one
systemctl daemon-reload
systemctl disable --now tailord.service 2>/dev/null || true
rm -f /etc/systemd/system/tailord.service
systemctl enable --now kbd-rgbd.service 2>/dev/null || true

echo ""
echo "Installation complete."
echo "  - kbd-rgbd.service is enabled and started"
echo "  - clevo-wmi.conf loaded at boot"
echo "  - kbd-brightness-up, kbd-brightness-down, kbd-preset-switch,"
echo "    kbd-preset-list, kbd-off available in /usr/local/bin/"
echo ""
echo "Next steps:"
echo "  1. If not already applied, patch DMI: sudo ./scripts/apply-dmi-patch.sh"
echo "  2. Reboot or run:    sudo modprobe clevo_wmi"
echo "  3. Bind keys in Hyprland to kbd-brightness-up/down and kbd-preset-switch"
