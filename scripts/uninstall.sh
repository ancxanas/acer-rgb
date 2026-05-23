#!/bin/sh
# Remove acer-rgb system-wide configuration
# Requires root privileges

set -e

if [ "$(id -u)" -ne 0 ]; then
  echo "This script must be run as root. Use: sudo $0"
  exit 1
fi

echo "Stopping and disabling services..."
systemctl stop tailord.service 2>/dev/null || true
systemctl disable tailord.service 2>/dev/null || true

echo "Removing systemd units..."
rm -f /etc/systemd/system/tailord.service
rm -f /etc/systemd/system/kbd-preset@.service

echo "Removing modprobe config..."
rm -f /etc/modules-load.d/clevo-wmi.conf
rm -f /etc/modprobe.d/tuxedo-keyboard.conf

echo "Removing presets..."
rm -rf /etc/tailord/keyboard
rm -rf /etc/tailord/profiles
rm -f /etc/tailord/active_profile.json

echo "Removing kbd-preset command..."
rm -f /usr/local/bin/kbd-preset

systemctl daemon-reload

echo ""
echo "Uninstallation complete."
echo ""
echo "Note: The DKMS patch was not reverted."
echo "To revert: edit the module source or reinstall tuxedo-drivers."
echo "  sudo dnf reinstall tuxedo-drivers"
