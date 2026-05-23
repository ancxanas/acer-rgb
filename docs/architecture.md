# Acer Keyboard Backlight — Full Configuration Reference

## Architecture Overview

```
Hardware (EC firmware)
  ↓ Fn keys control at EC level (always works independently)

tuxedo_keyboard.ko (kernel module, framework)
  ↓ clevo_wmi.ko (kernel module, interface)
    ↓ creates LED class device
      /sys/class/leds/rgb:kbd_backlight/

tailord (userspace daemon, from tuxedo-rs)
  ↓ reads /etc/tailord/*.json profiles
  ↓ writes to /sys/class/leds/rgb:kbd_backlight/multi_intensity
  ↓ handles suspend/resume, profile switching, D-Bus commands
```

---

## 1. Kernel Module Modes (Hardware-Level)

These are sent via WMI/ACPI to the hardware. **Only work on actual Clevo/TUXEDO hardware.** On our Acer, the WMI commands are no-ops (EC ignores them). Included for reference.

Set via `/etc/modprobe.d/tuxedo-keyboard.conf`:
```
options tuxedo_keyboard kbd_backlight_mode=2
```

| Key | Hex Value | Name | Description |
|-----|-----------|------|-------------|
| 0 | `0x00000000` | `CUSTOM` | Manual color control (default) |
| 1 | `0x1002a000` | `BREATHE` | Breathing/pulsing |
| 2 | `0x33010000` | `CYCLE` | Auto color cycling |
| 3 | `0x80000000` | `DANCE` | Dance/reactive pattern |
| 4 | `0xA0000000` | `FLASH` | Flashing/strobe |
| 5 | `0x70000000` | `RANDOM_COLOR` | Random color jumps |
| 6 | `0x90000000` | `TEMPO` | Tempo/music reactive |
| 7 | `0xB0000000` | `WAVE` | Wave pattern |

```bash
# Read current value
cat /sys/module/tuxedo_keyboard/parameters/kbd_backlight_mode
```

---

## 2. tailord Profile Format

All profiles under `/etc/tailord/`.

### Active Profile Selector

`/etc/tailord/active_profile.json` — symlink to a profile:
```json
{
  "fans": ["default", "default"],
  "leds": [{
    "device_name": "platform:tuxedo_keyboard",
    "function": "kbd_backlight",
    "profile": "default",
    "mode": "Rgb"
  }],
  "performance_profile": "performance"
}
```

| Field | Values | Description |
|-------|--------|-------------|
| `fans` | `["default", "default"]` | Fan profile names |
| `leds[].device_name` | `"platform:tuxedo_keyboard"` | Matches LED device parent |
| `leds[].function` | `"kbd_backlight"` | Matches LED function name |
| `leds[].profile` | `"default"` | References `/etc/tailord/keyboard/default.json` |
| `leds[].mode` | `"Rgb"` or `"Monochrome"` | Color mode |
| `performance_profile` | `"performance"` | CPU/GPU performance profile |

### LedControllerMode

| Mode | Description |
|------|-------------|
| `"Rgb"` | Multicolor — writes `R G B` to `multi_intensity` |
| `"Monochrome"` | Single-color — writes to `brightness` only |

---

## 3. Color Profile Format

`/etc/tailord/keyboard/default.json`

### ColorProfile Types

| JSON key | Behavior |
|----------|----------|
| `"None"` | Off, no animation |
| `"Single"` | Static single color |
| `"Multiple"` | Sequence loop with transitions |

### ColorPoint Fields

| Field | Type | Values | Description |
|-------|------|--------|-------------|
| `color` | object | `{ "r": 0-255, "g": 0-255, "b": 0-255 }` | Target RGB |
| `transition` | string | `"Linear"` or `"None"` | How to reach this color |
| `transition_time` | integer | milliseconds | Duration |

Transition behavior:

| transition | time | What happens |
|------------|------|-------------|
| `"None"` | `> 0` | Snap instantly, hold N ms |
| `"None"` | `0` | Snap and hold forever |
| `"Linear"` | `> 0` | Smooth fade over N ms |
| `"Linear"` | `0` | Snap (same as None with 0) |

### Animation Loop

Points are iterated in order. After the last point, it wraps to the first. The "previous color" for point 0 is the last point's color (circular interpolation).

```
Last point ──transition──→ Point 0 ──→ Point 1 ──→ ... ──→ Last point ──→ (repeat)
```

---

## 4. Profile Examples

### Rainbow Cycle (6 colors, smooth)
```json
{
  "Multiple": [
    { "color": { "r": 255, "g": 0,   "b": 0   }, "transition": "Linear", "transition_time": 4000 },
    { "color": { "r": 255, "g": 165, "b": 0   }, "transition": "Linear", "transition_time": 4000 },
    { "color": { "r": 255, "g": 255, "b": 0   }, "transition": "Linear", "transition_time": 4000 },
    { "color": { "r": 0,   "g": 255, "b": 0   }, "transition": "Linear", "transition_time": 4000 },
    { "color": { "r": 0,   "g": 0,   "b": 255 }, "transition": "Linear", "transition_time": 4000 },
    { "color": { "r": 148, "g": 0,   "b": 211 }, "transition": "Linear", "transition_time": 4000 }
  ]
}
```
24-second total cycle.

### Warm Ambient (slow fade)
```json
{
  "Multiple": [
    { "color": { "r": 255, "g": 180, "b": 80  }, "transition": "Linear", "transition_time": 15000 },
    { "color": { "r": 200, "g": 100, "b": 50  }, "transition": "Linear", "transition_time": 15000 },
    { "color": { "r": 255, "g": 220, "b": 150 }, "transition": "Linear", "transition_time": 15000 },
    { "color": { "r": 180, "g": 80,  "b": 40  }, "transition": "Linear", "transition_time": 15000 }
  ]
}
```
1-minute slow ambient.

### Static Colors
```json
{ "Single": { "r": 255, "g": 200, "b": 100 } }   ← warm white
{ "Single": { "r": 255, "g": 0,   "b": 0   } }   ← red
"None"                                              ← off
```

### Strobe (fast)
```json
{
  "Multiple": [
    { "color": { "r": 255, "g": 255, "b": 255 }, "transition": "None", "transition_time": 100 },
    { "color": { "r": 0,   "g": 0,   "b": 0   }, "transition": "None", "transition_time": 100 }
  ]
}
```
~5Hz strobe.

### Police Lights
```json
{
  "Multiple": [
    { "color": { "r": 255, "g": 0, "b": 0   }, "transition": "Linear", "transition_time": 300 },
    { "color": { "r": 0,   "g": 0, "b": 255 }, "transition": "Linear", "transition_time": 300 }
  ]
}
```

### Fast Snap Cycle
```json
{
  "Multiple": [
    { "color": { "r": 255, "g": 0,   "b": 0   }, "transition": "None", "transition_time": 500 },
    { "color": { "r": 0,   "g": 255, "b": 0   }, "transition": "None", "transition_time": 500 },
    { "color": { "r": 0,   "g": 0,   "b": 255 }, "transition": "None", "transition_time": 500 },
    { "color": { "r": 255, "g": 0,   "b": 255 }, "transition": "None", "transition_time": 500 },
    { "color": { "r": 0,   "g": 255, "b": 255 }, "transition": "None", "transition_time": 500 },
    { "color": { "r": 255, "g": 255, "b": 0   }, "transition": "None", "transition_time": 500 }
  ]
}
```

---

## 5. Color Reference

| Color | R | G | B | Hex |
|-------|---|---|---|-----|
| Black (off) | 0 | 0 | 0 | #000000 |
| White | 255 | 255 | 255 | #FFFFFF |
| Warm White | 255 | 200 | 100 | #FFC864 |
| Cool White | 200 | 220 | 255 | #C8DCFF |
| Red | 255 | 0 | 0 | #FF0000 |
| Orange | 255 | 165 | 0 | #FFA500 |
| Yellow | 255 | 255 | 0 | #FFFF00 |
| Green | 0 | 255 | 0 | #00FF00 |
| Cyan | 0 | 255 | 255 | #00FFFF |
| Blue | 0 | 0 | 255 | #0000FF |
| Purple | 128 | 0 | 128 | #800080 |
| Magenta | 255 | 0 | 255 | #FF00FF |
| Pink | 255 | 192 | 203 | #FFC0CB |

---

## 6. Commands

```bash
# Restart tailord after profile change
sudo systemctl restart tailord.service

# Read current color
cat /sys/class/leds/rgb:kbd_backlight/multi_intensity

# Set color manually (bypasses tailord)
echo "255 100 0" | sudo tee /sys/class/leds/rgb:kbd_backlight/multi_intensity

# Toggle on/off
echo 255 | sudo tee /sys/class/leds/rgb:kbd_backlight/brightness   # on
echo 0    | sudo tee /sys/class/leds/rgb:kbd_backlight/brightness   # off

# Check tailord logs
sudo journalctl -u tailord.service -n 50 --no-pager

# List all LED devices
ls /sys/class/leds/

# Check modules
lsmod | grep -E "tuxedo|clevo"
```

---

## 7. Multi-Profile Switching

Create a second keyboard profile:

`/etc/tailord/keyboard/amber.json`:
```json
{ "Single": { "r": 255, "g": 120, "b": 0 } }
```

Then a profile selector:

`/etc/tailord/profiles/amber.json`:
```json
{
  "fans": ["default", "default"],
  "leds": [{
    "device_name": "platform:tuxedo_keyboard",
    "function": "kbd_backlight",
    "profile": "amber",
    "mode": "Rgb"
  }],
  "performance_profile": "performance"
}
```

Switch:
```bash
sudo ln -sf /etc/tailord/profiles/amber.json /etc/tailord/active_profile.json
sudo systemctl restart tailord.service
```

---

## 8. Animation Engine Internals

tailord pre-computes all frames at startup:

1. `calculate_color_animation_steps()` — builds frame list:
   - `Linear`: interpolates RGB in 80ms max step size (12.5 FPS)
   - Adaptive: larger color diffs = more steps, longer time = finer grain
   - Minimum perceptible delta: 15 RGB units/sec
   - Example: Red→Green over 6s ≈ 59 steps at ~10 FPS
   - `None`: single frame with hold time

2. `run_color_animation()` — infinite `.cycle()` loop:
   - Writes each frame to `multi_intensity`
   - Sleeps frame duration
   - Pauses during suspend
   - Listens for profile/color override via channels

---

## 9. LED Sysfs Files

`/sys/class/leds/rgb:kbd_backlight/`:

| File | Access | Description |
|------|--------|-------------|
| `brightness` | RW | Master 0-255 |
| `max_brightness` | RO | Always 255 |
| `multi_intensity` | RW | `"R G B"` space-separated |
| `multi_index` | RO | `"red green blue"` |
| `trigger` | RW | LED triggers (none, timer, heartbeat, etc.) |

---

## 10. File Inventory

| File | Purpose |
|------|---------|
| `/etc/tailord/active_profile.json` | Symlink to active profile |
| `/etc/tailord/profiles/default.json` | Active profile definition |
| `/etc/tailord/keyboard/default.json` | **Keyboard animation profile (edit this)** |
| `/etc/tailord/fan/default.json` | Fan curve |
| `/etc/systemd/system/tailord.service` | tailord systemd service |
| `/usr/bin/tailord` | Built from tuxedo-rs |
| `/usr/share/dbus-1/system.d/com.tux.Tailor.conf` | D-Bus policy |
| `/etc/modules-load.d/clevo-wmi.conf` | Auto-load clevo_wmi |
| `/etc/modprobe.d/tuxedo-keyboard.conf` | Module parameters |
| `/usr/src/tuxedo-drivers-4.22.1/tuxedo_compatibility_check/` | Patched DMI check |

---

*Updated May 23, 2026*
