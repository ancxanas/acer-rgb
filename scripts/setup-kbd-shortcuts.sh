#!/bin/sh
# Setup keyboard brightness shortcuts in KDE Plasma 6
# Uses desktop file shortcuts -> kbd-brightness CLI (PowerDevil D-Bus, shows OSD, no sudo)
# Run this from your user account (DO NOT use sudo)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "Installing shortcut desktop files..."
mkdir -p "$HOME/.local/share/kglobalaccel"
cp "$SCRIPT_DIR/packaging/kglobalaccel/kbd-brightness-up.desktop" \
   "$HOME/.local/share/kglobalaccel/"
cp "$SCRIPT_DIR/packaging/kglobalaccel/kbd-brightness-down.desktop" \
   "$HOME/.local/share/kglobalaccel/"

echo "Registering shortcuts with KDE..."
kwriteconfig6 --file kglobalshortcutsrc \
  --group "kbd-brightness-up.desktop" \
  --key "_k_friendly_name" "Keyboard Brightness Up"
kwriteconfig6 --file kglobalshortcutsrc \
  --group "kbd-brightness-up.desktop" \
  --key "_launch" "Meta+K,none,Keyboard Brightness Up"

kwriteconfig6 --file kglobalshortcutsrc \
  --group "kbd-brightness-down.desktop" \
  --key "_k_friendly_name" "Keyboard Brightness Down"
kwriteconfig6 --file kglobalshortcutsrc \
  --group "kbd-brightness-down.desktop" \
  --key "_launch" "Ctrl+Shift+K,none,Keyboard Brightness Down"

# Disable dead PowerDevil shortcuts (no physical keyboard brightness keys on this hardware)
kwriteconfig6 --file kglobalshortcutsrc \
  --group "org_kde_powerdevil" \
  --key "Increase Keyboard Brightness" \
  "none,none,Increase Keyboard Brightness"
kwriteconfig6 --file kglobalshortcutsrc \
  --group "org_kde_powerdevil" \
  --key "Decrease Keyboard Brightness" \
  "none,none,Decrease Keyboard Brightness"
kwriteconfig6 --file kglobalshortcutsrc \
  --group "org_kde_powerdevil" \
  --key "Toggle Keyboard Backlight" \
  "none,none,Toggle Keyboard Backlight"

echo "Activating shortcuts..."
systemctl --user restart plasma-kglobalaccel.service 2>/dev/null || true

echo ""
echo "Done! Shortcuts:"
echo "  Win+K          -> keyboard brightness up (with OSD)"
echo "  Ctrl+Shift+K   -> keyboard brightness down (with OSD)"
echo ""
echo "You can customize these in System Settings > Keyboard > Shortcuts > Custom Shortcuts"
