# acer-rgb

Keyboard backlight enablement for Acer Aspire A715-79G (and similar) using TUXEDO Computers kernel drivers with a DMI bypass patch.

## What it does

- Loads `clevo_wmi` + `tuxedo_keyboard` kernel modules at boot (patched to bypass DMI hardware check)
- Creates `/sys/class/leds/rgb:kbd_backlight/` LED class device
- Runs `tailord` daemon to animate RGB colors via sysfs
- Provides 16 preset color profiles and a `kbd-preset` command to switch between them

## Quick start

```bash
# 1. Install system files (presets, services, command)
sudo ./scripts/install-system.sh

# 2. Apply DMI bypass patch (kernel modification — opt in)
sudo ./scripts/apply-dmi-patch.sh

# 3. Load modules
sudo modprobe clevo_wmi

# 4. Start daemon
sudo systemctl start tailord.service

# 5. Switch presets
kbd-preset rainbow
kbd-preset list
```

## Presets

| Name | Description |
|------|-------------|
| `rainbow` | 6-color smooth rainbow, 4s per transition |
| `cycle` | Red → Green → Blue, 6s each |
| `warm-ambient` | Slow warm-tone fades, 15s each |
| `pastel` | Soft pastels, 5s each |
| `ocean` | Blue and teal tones, 6s each |
| `sunset` | Orange, red, purple warm tones, 5s each |
| `snap-cycle` | 6 colors instant snap, 500ms each |
| `strobe` | White/black 100ms strobe |
| `police` | Red/blue alternating, 300ms |
| `static-warmwhite` | Static warm white (255, 200, 100) |
| `static-red` | Static red |
| `static-blue` | Static blue |
| `static-green` | Static green |
| `static-purple` | Static purple |
| `off` | Turns backlight off |

## Files

```
├── presets/keyboard/     ← animation JSON definitions
├── presets/profiles/     ← profile selectors
├── packaging/systemd/    ← systemd service templates
├── packaging/modules-load.d/  ← kernel module auto-load
├── packaging/modprobe.d/      ← module parameters
├── patches/              ← DMI bypass patch
├── scripts/
│   ├── kbd-preset             ← preset switching command
│   ├── install-system.sh      ← system installation
│   ├── uninstall.sh           ← system removal
│   └── apply-dmi-patch.sh     ← DKMS patch application
└── docs/
    ├── investigation.md       ← full debugging story
    └── architecture.md        ← configuration reference
```

## Requirements

- Fedora 44+ (other distros: adapt paths)
- `tuxedo-drivers` package from the official TUXEDO repository
- `tailord` from tuxedo-rs (built from source)
- DKMS patches rebuild after kernel updates

## Uninstall

```bash
sudo ./scripts/uninstall.sh
```

To revert the DMI patch:

```bash
sudo dnf reinstall tuxedo-drivers
```

## License

GPL-2.0 (matches tuxedo-drivers license)
