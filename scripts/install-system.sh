#!/bin/sh
# Install acer-rgb system-wide configuration
# Requires root privileges

set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "This script must be run as root. Use: sudo $0"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
echo "Installing from: $SCRIPT_DIR"

# Copy presets
echo "  presets/keyboard/*  -> /etc/tailord/keyboard/"
install -d -m755 /etc/tailord/keyboard /etc/tailord/profiles
cp -a "$SCRIPT_DIR/presets/keyboard/"*.json /etc/tailord/keyboard/
install -m644 "$SCRIPT_DIR/presets/profiles/"*.json /etc/tailord/profiles/

# Copy systemd units
echo "  packaging/systemd/  -> /etc/systemd/system/"
install -m644 "$SCRIPT_DIR/packaging/systemd/tailord.service" /etc/systemd/system/tailord.service
install -m644 "$SCRIPT_DIR/packaging/systemd/kbd-preset@.service" /etc/systemd/system/kbd-preset@.service

# Copy modprobe configs
echo "  packaging/modules-load.d/  -> /etc/modules-load.d/"
install -m644 "$SCRIPT_DIR/packaging/modules-load.d/clevo-wmi.conf" /etc/modules-load.d/clevo-wmi.conf

echo "  packaging/modprobe.d/  -> /etc/modprobe.d/"
install -m644 "$SCRIPT_DIR/packaging/modprobe.d/tuxedo-keyboard.conf" /etc/modprobe.d/tuxedo-keyboard.conf

# Install kbd-preset command
echo "  scripts/kbd-preset  -> /usr/local/bin/kbd-preset"
install -m755 "$SCRIPT_DIR/scripts/kbd-preset" /usr/local/bin/kbd-preset

# Reload systemd and enable services
systemctl daemon-reload
systemctl enable tailord.service 2>/dev/null || true

# Ensure active symlink exists (default to cycle if not set)
if [ ! -L /etc/tailord/active_profile.json ]; then
  ln -sf /etc/tailord/profiles/cycle.json /etc/tailord/active_profile.json
fi

echo ""
echo "Installation complete."
echo "  - tailord.service is enabled"
echo "  - clevo-wmi.conf loaded at boot"
echo "  - kbd-preset command available"
echo ""
echo "Next steps:"
echo "  1. Apply DMI patch:  sudo ./scripts/apply-dmi-patch.sh"
echo "  2. Reboot or run:    sudo modprobe clevo_wmi"
echo "  3. Start tailord:    sudo systemctl start tailord.service"
echo "  4. Switch presets:   kbd-preset list"
