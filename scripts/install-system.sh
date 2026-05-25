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

# Copy modprobe configs
echo "  packaging/modules-load.d/  -> /etc/modules-load.d/"
install -m644 "$SCRIPT_DIR/packaging/modules-load.d/clevo-wmi.conf" /etc/modules-load.d/clevo-wmi.conf

echo "  packaging/modprobe.d/  -> /etc/modprobe.d/"
install -m644 "$SCRIPT_DIR/packaging/modprobe.d/tuxedo-keyboard.conf" /etc/modprobe.d/tuxedo-keyboard.conf

# Install kbdctl command
echo "  scripts/kbdctl  -> /usr/local/bin/kbdctl"
install -m755 "$SCRIPT_DIR/scripts/kbdctl" /usr/local/bin/kbdctl

# Remove old commands
rm -f /usr/local/bin/kbd-brightness /usr/local/bin/kbd-preset

# Install KDE global shortcut desktop files (source for kbdctl setup)
echo "  packaging/kglobalaccel/  -> /usr/local/share/acer-rgb/kglobalaccel/"
install -d -m755 /usr/local/share/acer-rgb/kglobalaccel
install -m644 "$SCRIPT_DIR/packaging/kglobalaccel/"*.desktop /usr/local/share/acer-rgb/kglobalaccel/

# Reload systemd and enable services
systemctl daemon-reload
systemctl enable tailord.service 2>/dev/null || true

# Ensure active symlink exists (default to cycle if not set)
if [ ! -L /etc/tailord/active_profile.json ]; then
  ln -sf /etc/tailord/profiles/cycle.json /etc/tailord/active_profile.json
fi

# Auto-setup KDE global shortcuts for the user who ran sudo
if [ -n "$SUDO_USER" ] && [ -d "/home/$SUDO_USER" ]; then
  echo ""
  echo "  Setting up KDE shortcuts for $SUDO_USER..."
  runuser -u "$SUDO_USER" /usr/local/bin/kbdctl setup 2>/dev/null || true
fi

echo ""
echo "Installation complete."
echo "  - tailord.service is enabled"
echo "  - clevo-wmi.conf loaded at boot"
echo "  - kbdctl command available"
echo ""
echo "Usage:"
echo "  kbdctl brightness up|down|set|get|toggle    (shows OSD)"
echo "  kbdctl preset list|switch <name>"
echo ""
echo "Next steps:"
echo "  1. Apply DMI patch:  sudo ./scripts/apply-dmi-patch.sh"
echo "  2. Reboot or run:    sudo modprobe clevo_wmi"
echo "  3. Start tailord:    sudo systemctl start tailord.service"
