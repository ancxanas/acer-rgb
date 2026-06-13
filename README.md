# acer-rgb

Keyboard backlight enablement for Acer Aspire A715-79G (and similar) using
TUXEDO Computers kernel drivers with a DMI bypass patch.

## What it does

- Loads `clevo_wmi` + `tuxedo_keyboard` kernel modules at boot (patched to
  bypass DMI hardware check)
- Creates `/sys/class/leds/rgb:kbd_backlight/` LED class device
- Runs `kbd-rgbd` — a minimal Rust daemon that animates RGB colors via sysfs
- Provides 16 preset color profiles and shell scripts to control them
- No D-Bus, no KDE dependencies, no Python

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  kbd-preset-switch               │
│                  kbd-preset-list                 │
│                  kbd-off                         │
│                  kbd-brightness-up/down          │
└───────┬─────────────────────────────┬────────────┘
        │ write /run/kbd-rgbd/cmd      │ write sysfs
        ▼                              ▼
┌────────────────┐           ┌──────────────────┐
│   kbd-rgbd     │  reads    │  multi_intensity │
│  (Rust daemon) │◄─────────►│  brightness      │
│                │  writes   │  (LED class)     │
└───┬───┬───┬────┘           └──────────────────┘
    │   │   │
    │   │   └── /etc/tailord/keyboard/*.json
    │   │        (animation definitions)
    │   │
    │   └────── /etc/tailord/profiles/*.json
    │            (profile selectors)
    │
    └────────── /etc/tailord/active_profile.json
                 (symlink — atomically swapped)
```

## Quick start

```bash
# 1. Build and install system-wide
sudo ./scripts/install-system.sh

# 2. Apply DMI bypass patch (kernel modification — opt in)
sudo ./scripts/apply-dmi-patch.sh

# 3. Load modules
sudo modprobe clevo_wmi

# 4. Use it
kbd-preset-list
kbd-preset-switch
kbd-brightness-up
kbd-brightness-down
kbd-off
```

## Usage

| Command | Description |
|---------|-------------|
| `kbd-brightness-up` | Increase brightness by ~10% |
| `kbd-brightness-down` | Decrease brightness by ~10% |
| `kbd-preset-switch` | Cycle to next preset |
| `kbd-preset-list` | List all presets (active marked with `*`) |
| `kbd-off` | Turn off backlight (daemon stays alive) |

Brightness and animations are independent — brightness scales the LED
class output without affecting the daemon's RGB animation.

### Hyprland keybinds example

Add to `~/.config/hypr/hyprland.conf`:

```
bind = , XF86KbdBrightnessUp, exec, kbd-brightness-up
bind = , XF86KbdBrightnessDown, exec, kbd-brightness-down
bind = , XF86KbdLightOnOff, exec, kbd-off
bind = $mod+KB, KB, exec, kbd-preset-switch
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
| `off` | Turns backlight off |

## Files

```
├── src/
│   ├── main.rs                 ← entrypoint: calls daemon::run()
│   ├── lib.rs                  ← crate root + re-exports
│   ├── error.rs                ← KbdError enum + From impls
│   ├── types.rs                ← JSON types + parsers + 8 tests
│   ├── animation.rs            ← lerp() + build_frames() + 2 tests
│   └── runtime.rs              ← daemon runtime + 1 test
├── tests/
│   └── profile_loading.rs      ← 2 integration tests
├── .github/workflows/ci.yml     ← CI: fmt, clippy, test, build --release
├── Cargo.toml
├── presets/keyboard/           ← animation JSON definitions
├── presets/profiles/           ← profile selectors
├── packaging/
│   ├── kbd-rgbd.service        ← systemd service unit
│   ├── modules-load.d/         ← kernel module auto-load
│   └── modprobe.d/             ← module parameters
├── scripts/
│   ├── kbd-brightness-up       ← increase backlight
│   ├── kbd-brightness-down     ← decrease backlight
│   ├── kbd-preset-switch       ← cycle to next preset
│   ├── kbd-preset-list         ← list presets
│   ├── kbd-off                 ← turn off (daemon stays alive)
│   ├── install-system.sh       ← system installation
│   ├── uninstall.sh            ← system removal
│   └── apply-dmi-patch.sh      ← DKMS patch application
├── patches/                    ← DMI bypass patch
└── docs/
    ├── investigation.md        ← full debugging story
    └── architecture.md         ← configuration reference
```

## Requirements

- Linux with systemd v240+ (for `RuntimeDirectory=` support)
- `tuxedo-drivers` package from the official TUXEDO repository
- Rust toolchain (for building `kbd-rgbd`)
- DKMS patches rebuild after kernel updates

## Daemon commands

Write to `/run/kbd-rgbd/cmd` (newline-terminated):

| Command | Effect |
|---------|--------|
| `stop` | Write `0 0 0` to sysfs, exit |
| `reload` | Reload current profile from disk |
| `profile <name>` | Switch to profile (atomically updates symlink) |
| `brightness_up` | Increase brightness by ~10% (+26, clamped 0–255) |
| `brightness_down` | Decrease brightness by ~10% (-26, clamped 0–255) |

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
