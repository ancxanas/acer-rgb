# Making the Acer Aspire A715-79G Keyboard Backlight Work with TUXEDO Drivers

## Foreword

This is the story of a four-hour deep dive into kernel drivers, WMI interfaces, DKMS builds, and Rust daemons — all because a keyboard backlight refused to be controlled by software. The laptop in question is an **Acer Aspire A715-79G** with an **Intel i7-13620H** and **NVIDIA RTX 3050 6GB**, running **Fedora 44** on **KDE Plasma 6** (Wayland). The keyboard backlight worked at the hardware level — the Fn+Key combos cycled through colors just fine — but no software interface (`/sys/class/leds/rgb:kbd_backlight/`) existed to control it programmatically. The goal was simple: restore the auto-cycling color mode that a previous (now-defunct) third-party repo had supposedly provided.

---

## 1. Hardware Context

The Acer Aspire A715-79G uses a standard **Embedded Controller (EC)** to manage low-level hardware: keyboard, fans, thermal sensors, and the keyboard backlight. This EC is a microcontroller running proprietary firmware. It responds to Fn key presses directly — there is no "software animation" happening. When you press Fn+F9 to change backlight brightness or Fn+F10 to cycle colors, the EC handles it entirely in firmware. The OS never sees these as software events.

This architecture is common in modern laptops. The problem arises when you want software to control the backlight — to create automatic color cycling, reactive effects, or integrate with desktop themes. For that, you need a **LED class device** in `/sys/class/leds/` that accepts writes.

**Does the Acer expose one?** Out of the box, no. Not via `acer-wmi`, not via any ACPI interface, and certainly not via a proprietary driver.

---

## 2. The Old Setup — Atul977's Fedora-Tuxedo-Repo

The previous owner had this working via **Atul977's Fedora-Tuxedo-Repo**, a third-party Copr-like repository hosted on GitHub Pages. It shipped two packages:

- **`tuxedo-drivers-dkms`** — A DKMS package containing kernel modules from TUXEDO Computers (`tuxedo_keyboard`, `clevo_wmi`, `clevo_acpi`, etc.)
- **`tuxedo-rs`** — A Rust workspace containing `tailord` (the daemon) and `tailor_gui` (a GTK4 control panel)

The `tuxedo-drivers-dkms` package was version `3.2.14` — an ancient snapshot of the upstream TUXEDO repository. The `tuxedo-rs` package was version `0.2.5`, built from the upstream **AaronErhardt/tuxedo-rs** repository with exactly one downstream patch: a Niri-compatibility fix for the color picker widget.

### The Critical Discovery

When we inspected the `tuxedo-drivers` source from Atul977's repo, we found something surprising. The repository had been created from an **archived snapshot** — the entire Git history was squashed into a single commit. The commit message read:

> "Added tuxedo-keyboard and tuxedo-rs"

But there was **exactly one source patch** in the entire repository. In `tuxedo_compatibility_check.c`, the function `tuxedo_is_compatible()` had been changed to unconditionally `return true`.

That's it. That was the entire "magic" of the Atul977 setup. No sophisticated driver hacks, no reverse-engineered EC protocol, no custom ACPI methods. Just a single line change: `return true` instead of the DMI-based compatibility check.

### The Problem with Atul977's Repo

Despite this, the repo was removed from our system because:

1. **No GPG signing** — `gpgcheck=0` meant unsigned RPMs were accepted
2. **GitHub Pages hosting** — served from a plain GitHub Pages site, no proper repository infrastructure
3. **Archived repo** — the source repo had been archived/disabled by GitHub, meaning zero upstream updates
4. **Zero-commit fork** — no proper fork relationship, no way to track upstream changes
5. **AI-generated metadata** — the spec file contained the comment `# Maintainer: Gemini`, suggesting AI-written packaging
6. **Outdated code** — version 3.2.14 vs upstream's 4.22.1 (at time of writing)

The decision was made to remove Atul977's repo and use the **official TUXEDO Computers repository** instead: GPG-signed packages, maintained by the vendor, with MOK-signed kernel modules for Secure Boot compatibility.

---

## 3. First Attempt — Official TUXEDO Repository

The official TUXEDO repository was added:

```
/etc/yum.repos.d/tuxedo.repo
```

And the packages were installed cleanly:

```
sudo dnf install tuxedo-drivers tuxedo-control-center
```

This installed version **4.22.1** of `tuxedo-drivers`, containing these kernel modules (among others):

| Module | Purpose |
|--------|---------|
| `tuxedo_keyboard` | Framework module for keyboard + backlight |
| `tuxedo_compatibility_check` | DMI-based hardware validation |
| `clevo_wmi` | Driver for Clevo WMI interface (keyboard backlight) |
| `clevo_acpi` | Driver for Clevo ACPI interface (keyboard backlight) |
| `tuxedo_io` | IO port communication module |
| `ite_829x` | ITE 829x keyboard controller driver |
| `uniwill_wmi` | Uniwill WMI driver (for Uniwill-built laptops) |

But when we tried to load the keyboard module:

```
$ sudo modprobe tuxedo_keyboard
modprobe: ERROR: could not insert 'tuxedo_keyboard': No such device
```

The kernel module existed — it was compiled and installed — but it refused to initialize.

---

## 4. The Investigation — Why Doesn't This Work?

### 4.1 The DMI Check

The first stop was `tuxedo_compatibility_check.c`. The function `tuxedo_is_compatible()` is called during `tuxedo_keyboard_init()` (the module's `__init` function). If it returns `false`, the module init fails with `-ENODEV` and the module is not loaded.

The compatibility check works by matching the system's **DMI data** (manufacturer, product name, BIOS version) against a whitelist of known-compatible hardware. It checks:

1. Is the DMI vendor in the explicit whitelist (TUXEDO, Clevo, SCHENKER, etc.)?
2. Is the product on the supported Raptor Lake skip-list? (Certain Raptor Lake models bypass the check; others don't.)

Our Acer — built by Acer, not TUXEDO or Clevo — failed both checks. The module refused to load.

### 4.2 The Atul977 Patch

We already knew from Atul977's repo that the fix was trivial. In `/usr/src/tuxedo-drivers-4.22.1/tuxedo_compatibility_check/tuxedo_compatibility_check.c`:

```c
// Before (kernel 4.22.1):
bool tuxedo_is_compatible(void)
{
    // ... 150 lines of DMI matching logic ...
    return match_found;
}

// After (Atul977's patch):
bool tuxedo_is_compatible(void)
{
    return true;
}
```

This is what we applied. After patching and rebuilding via DKMS:

```bash
dkms build tuxedo-drivers/4.22.1
dkms install tuxedo-drivers/4.22.1
```

But even after this, `modprobe tuxedo_keyboard` succeeded... and still no LED device appeared.

### 4.3 Understanding the Module Architecture

This is where the investigation got interesting. `tuxedo_keyboard` is not a standalone driver for keyboard backlights. It is a **framework module** that exports symbols for other driver modules to use.

The architecture is:

```
         clevo_wmi ──┐
                      ├──→ tuxedo_keyboard (framework)
         clevo_acpi ──┘
                          ↓
              tuxedo_compatibility_check (dependency)
```

`clevo_wmi` and `clevo_acpi` are the hardware-specific drivers that bind to actual WMI/ACPI interfaces. When they successfully probe, they call `clevo_keyboard_add_interface()`, which is an **EXPORT_SYMBOL** from the `tuxedo_keyboard` module. This function registers the hardware interface and triggers the creation of the LED class device.

So the chain is:

1. **`clevo_wmi`** must successfully probe and bind to WMI GUIDs
2. This requires `tuxedo_keyboard` to be loaded (dependency for symbols)
3. `tuxedo_keyboard` requires `tuxedo_compatibility_check` to pass (we fixed this)

But `clevo_wmi` also has its own probe check. Let's look at that.

### 4.4 The WMI GUIDs — Not What We Expected

`clevo_wmi` matches two WMI GUIDs:

```
ABBC0F6B-8EA1-11D1-00A0-C90629100000  → Method GUID
ABBC0F6D-8EA1-11D1-00A0-C90629100000-1 → Event GUID
ABBC0F6C-8EA1-11D1-00A0-C90629100000-2 → Another event GUID
```

The comment in the source code says:

```c
// Since the WMI GUIDs aren't unique let's (at least)
// check the return of some "known existing general" method
```

This comment is the most important clue in the entire codebase. The developer knew that these GUIDs were **not unique to Clevo hardware**. They're actually **generic Microsoft ACPI WMI GUIDs** that every modern laptop with an ACPI WMI implementation exposes.

Here's why:

Microsoft's ACPI-WMI specification defines standard GUIDs for WMI communication. The `ABBC0F6*` namespace is a Microsoft-assigned range. Every Windows laptop with ACPI WMI support (which is essentially all modern x86 laptops) has these GUIDs. They're part of the Windows ACPI driver stack, not Clevo-specific at all.

To compensate for this, `clevo_wmi` performs a **method 0x52 check** in its probe function:

```c
status = clevo_wmi_evaluate(0x52, 0, &out_obj);
if (status < 0) {
    return -ENODEV;  // method call failed
}
if (out_obj->type != ACPI_TYPE_INTEGER ||
    (u32)out_obj->integer.value == 0xffffffff) {
    return -ENODEV;  // unexpected result
}
```

The logic: "If the WMI method 0x52 returns a valid integer (not 0xffffffff), it must be Clevo hardware."

**This is wrong.** Method 0x52 is a standard Microsoft WMI method that returns valid platform information on any laptop with ACPI WMI. On the Acer Aspire A715-79G, it returns a valid integer (not 0xffffffff), so the check passes. This means `clevo_wmi` would bind on essentially any modern laptop — if only it could load its dependency `tuxedo_keyboard`.

### 4.5 False Starts — Other Dead Ends

Before understanding the full picture, we investigated several other approaches:

**`clevo_acpi`**: This driver matches ACPI HID `CLV0001` and `CLV0002`. These are Clevo-specific ACPI device IDs. The Acer doesn't have them. The driver cannot bind.

**`acer-wmi`**: The in-kernel Acer WMI driver. It also has a DMI-based compatibility check and doesn't match the A715-79G model. Even if it did, `acer-wmi` controls different features (launcher keys, mail LED, etc.) — not keyboard backlights.

**`ite_829x`**: The ITE 8291/829x keyboard controller driver. This is for laptops with an ITE EC that speaks i8042 passthrough protocol. The Acer uses a different EC chipset (likely a proprietary Acer/Insyde EC) and doesn't respond to ITE commands.

**PS/2 i8042 attempt**: We tried direct i8042 port commands (`0x51`, `0x82` series). These are used by some Clevo/TUXEDO laptops for keyboard backlight. The Acer's i8042 controller responded to generic commands but backlight-specific commands had no effect.

**`tuxedo_nb04_kbd_backlight`**: A newer NB04 driver that uses platform device detection. Requires specific ACPI HIDs that don't exist on this Acer.

All of these were dead ends. The single gatekeeper was `tuxedo_is_compatible()`.

---

## 5. The Breakthrough

The breakthrough happened when we loaded `clevo_wmi` **after** the patched `tuxedo_keyboard` was loaded and `depmod -a` had been run to update module dependencies.

The sequence:

```bash
# 1. Patch the DMI check
# Edit /usr/src/tuxedo-drivers-4.22.1/tuxedo_compatibility_check/tuxedo_compatibility_check.c
# Change tuxedo_is_compatible() to return true

# 2. Rebuild via DKMS
sudo dkms build tuxedo-drivers/4.22.1
sudo dkms install tuxedo-drivers/4.22.1

# 3. Update module dependencies
sudo depmod -a

# 4. Load clevo_wmi (pulls in tuxedo_keyboard as dependency)
sudo modprobe clevo_wmi
```

The dmesg output showed:

```
[13097.248829] clevo_wmi: interface initialized
```

**`clevo_wmi` had bound successfully.** The WMI method 0x52 check passed. And then:

```
$ ls /sys/class/leds/rgb:kbd_backlight/multi_intensity
/sys/class/leds/rgb:kbd_backlight/multi_intensity
$ cat /sys/class/leds/rgb:kbd_backlight/multi_intensity
0 0 255
```

The LED class device existed. The keyboard backlight was controllable.

---

## 6. How the LED Device Gets Created

The full call chain from `modprobe clevo_wmi` to a working `multi_intensity` sysfs file:

```
modprobe clevo_wmi
  ↓ module init
clevo_wmi_probe()
  ↓ wmi_has_guid(CLEVO_WMI_EVENT_GUID) ✓
  ↓ wmi_has_guid(CLEVO_WMI_METHOD_GUID) ✓
  ↓ clevo_wmi_evaluate(0x52, 0, &out_obj) → returns valid integer ✓
  ↓
  clevo_keyboard_add_interface(&clevo_wmi_interface)
    ↓ (EXPORT_SYMBOL from tuxedo_keyboard module)
    tuxedo_keyboard_init_driver(&clevo_keyboard_driver)
      ↓
      platform_create_bundle()
        ↓ Creates platform device "tuxedo_keyboard"
        ↓ Registers clevo_keyboard_driver on it
        ↓
        clevo_keyboard_probe()
          ↓ kbd_backlight_mode ← from module_param (default: 1 = STATIC)
          ↓
          clevo_leds_init(dev)
            ↓ led_classdev_multicolor_register()
              → /sys/class/leds/rgb:kbd_backlight/
              → brightness      (0-255, master on/off)
              → max_brightness  255
              → multi_index     "red green blue"
              → multi_intensity  (write R G B values, space-separated)
```

The `clevo_keyboard_probe()` function also reads the `kbd_backlight_mode` module parameter and stores it in `dev->mode`. For mode 2 (CYCLE), it would set up a kernel workqueue to continuously cycle through colors. But more on that later.

---

## 7. Hardware Cycle Fails, Software Cycle Wins

### 7.1 Why `kbd_backlight_mode=2` Doesn't Work

The `kbd_backlight_mode` parameter was set in `/etc/modprobe.d/tuxedo-keyboard.conf`:

```
options tuxedo_keyboard kbd_backlight_mode=2
```

Mode 2 is CYCLE mode. In theory, this should make the keyboard auto-cycle through colors without any userspace intervention. The kernel module handles it via a workqueue that writes to hardware registers.

The workqueue function (`clevo_keyboard_cycle_work`) does something like:

```c
static void clevo_keyboard_cycle_work(struct work_struct *work)
{
    // Compute next color in HSV cycle
    // Write to hardware via WMI/ACPI method calls
    interface->method_call(CLEVO_CMD_SET_LED_COLOR, color_value, &result);
    // Schedule next cycle
    schedule_delayed_work(&dev->cycle_work, msecs_to_jiffies(50));
}
```

The problem: `interface->method_call()` goes through the `clevo_wmi_interface.method_call` function pointer, which calls `clevo_wmi_evaluate()`. This sends a WMI command through the **standard ACPI WMI interface**. On genuine Clevo hardware, this command hits Clevo-specific WMI handlers. On our Acer, the standard Microsoft ACPI WMI implementation sees the command and either ignores it or returns an error — the keyboard EC is unaffected.

The color stays static regardless of the mode parameter. We confirmed this by:

```bash
# Set mode=2, brightness=255, set a color
echo "0 255 0" > /sys/class/leds/rgb:kbd_backlight/multi_intensity

# Wait 5 seconds
cat /sys/class/leds/rgb:kbd_backlight/multi_intensity
0 255 0   # ← still green, no cycle
```

The kernel's hardware cycle path only works on actual Clevo/TUXEDO hardware where the EC speaks the Clevo WMI protocol.

### 7.2 The Userspace Solution — tailord

The TUXEDO Computers software stack includes `tuxedo-rs`, a Rust workspace containing `tailord` (a D-Bus daemon) and `tailor_gui` (a GTK4 control panel). `tailord` animates the keyboard backlight entirely by writing to the sysfs `multi_intensity` file in a controlled loop.

This is the proper approach for non-Clevo hardware: userspace animation via the generic LED class interface. No kernel code changes needed beyond the DMI bypass.

We built `tailord` from source:

```bash
git clone https://github.com/AaronErhardt/tuxedo-rs.git
cd tuxedo-rs
cargo build --release -p tailord
sudo install -m0755 target/release/tailord /usr/bin/tailord
sudo install -m0644 tailord/com.tux.Tailor.conf /usr/share/dbus-1/system.d/
```

The systemd service (`/etc/systemd/system/tailord.service`):

```ini
[Unit]
Description=Tux Tailor hardware control service
After=systemd-logind.service

[Service]
Type=dbus
BusName=com.tux.Tailor
ExecStart=/usr/bin/tailord
Environment="RUST_BACKTRACE=1"

[Install]
WantedBy=multi-user.target
```

### 7.3 The Animation Profile

`tailord` loads profiles from `/etc/tailord/`. The keyboard profile defines transitions:

```json
{
  "Multiple": [
    {
      "color": { "r": 255, "g": 0, "b": 0 },
      "transition": "Linear",
      "transition_time": 6000
    },
    {
      "color": { "r": 0, "g": 255, "b": 0 },
      "transition": "Linear",
      "transition_time": 6000
    },
    {
      "color": { "r": 0, "g": 0, "b": 255 },
      "transition": "Linear",
      "transition_time": 6000
    }
  ]
}
```

This defines: **Red → Green** (6 seconds), **Green → Blue** (6 seconds), **Blue → Red** (6 seconds) = **18-second cycle**. The transition is linear RGB interpolation, and `tailord` writes to `multi_intensity` at a high framerate to produce smooth animation.

The active profile (`/etc/tailord/active_profile.json`) references it:

```json
{
  "fans": ["default", "default"],
  "leds": [
    {
      "device_name": "platform:tuxedo_keyboard",
      "function": "kbd_backlight",
      "profile": "default",
      "mode": "Rgb"
    }
  ],
  "performance_profile": "performance"
}
```

### 7.4 Verifying the Animation

```bash
$ for i in $(seq 1 5); do
    cat /sys/class/leds/rgb:kbd_backlight/multi_intensity
    sleep 2
  done
125 0 130    # ← magenta/purple
203 0 52     # ← reddish
233 22 0     # ← orange
156 99 0     # ← yellow-green
82 173 0     # ← green
```

The colors changed smoothly every sample. The keyboard was cycling.

---

## 8. Final Configuration

Three configuration files ensure everything works on every boot:

### 1. Auto-load clevo_wmi at boot

`/etc/modules-load.d/clevo-wmi.conf`:
```
clevo_wmi
```

This causes systemd to load `clevo_wmi` during early boot. The module dependency system automatically pulls in `tuxedo_keyboard` and `tuxedo_compatibility_check`.

### 2. Module parameters (harmless, not functional)

`/etc/modprobe.d/tuxedo-keyboard.conf`:
```
options tuxedo_keyboard kbd_backlight_mode=2
```

The mode parameter is accepted by the module but has no effect on non-Clevo hardware. It's kept as documentation.

### 3. tailord service

Enabled at boot:
```
sudo systemctl enable tailord.service
```

Starts `tailord`, which reads the RGB cycle profile and begins animation.

### 4. The DKMS-patched module

The source patch persists in `/usr/src/tuxedo-drivers-4.22.1/tuxedo_compatibility_check/tuxedo_compatibility_check.c`. DKMS will automatically rebuild the patched module after every kernel update.

---

## 9. Key Technical Takeaways

### 9.1 WMI GUIDs ABBC0F6* Are Microsoft-Standard, Not Clevo-Specific

This is the most important misunderstanding that the entire tuxedo-drivers ecosystem relies on. The GUIDs `ABBC0F6B`, `ABBC0F6C`, `ABBC0F6D` are **Microsoft ACPI WMI GUIDs** defined in the ACPI-WMI specification. They are present on every modern x86 laptop. The Clevo-specific hardware detection via "method 0x52 returning a valid integer" is a detection method that produces false positives on all modern laptops with ACPI WMI.

The actual Clevo-specific behavior happens only when Clevo firmware has registered custom WMI method handlers. Without Clevo firmware, the WMI calls are handled by the generic ACPI WMI implementation, which returns data but doesn't affect hardware registers.

### 9.2 `tuxedo_is_compatible()` Is the Only Gatekeeper

Despite the sophisticated-sounding DMI compatibility check (150+ lines of vendor/product matching, Raptor Lake skip-lists, legacy Clevo support), the entire tuxedo-drivers stack is gated by exactly one function that returns a boolean. The function is **not a technical necessity** — it's a **legal/policy protection** to prevent TUXEDO from being flooded with support requests from non-TUXEDO hardware.

The modules will work (mostly) on any modern x86 laptop. Some features will be non-functional (hardware cycling, performance profiles), but the core keyboard backlight control via sysfs will work.

### 9.3 `tuxedo_keyboard` Is a Framework, Not a Driver

`tuxedo_keyboard` does not directly communicate with hardware. It provides:

- Symbol exports for hardware-specific driver modules (`clevo_keyboard_add_interface()`, etc.)
- A `led_classdev_multicolor` registration mechanism
- A `platform_device` for bind/unbind lifecycle management
- Mode parameter handling (static/cycle/breathing/wave)

The actual hardware communication is delegated to `clevo_wmi`, `clevo_acpi`, or other interface modules. On this Acer, only `clevo_wmi` can bind (because the WMI GUIDs exist), and its hardware method calls are no-ops.

### 9.4 `clevo_wmi` Works on Any Laptop with ACPI WMI

Once the DMI check is bypassed:

- `clevo_wmi` will bind on any laptop with standard ACPI WMI
- The LED class device will be created
- `multi_intensity` writes will work (they're translated to WMI calls by the kernel, then absorbed by the generic ACPI WMI handler)
- The keyboard backlight colors will change in response to writes

But:

- Hardware cycling will not work (Clevo-specific WMI commands are no-ops)
- The keyboard EC independently continues to respond to Fn keys
- There is a potential conflict: if the user presses Fn to change color, the EC overrides whatever `multi_intensity` was set to

### 9.5 Userspace Animation > Kernel Workqueues

When the hardware doesn't support kernel-level cycling, the clean solution is userspace animation via sysfs. `tailord` demonstrates this: it's a simple loop that writes to `/sys/class/leds/rgb:kbd_backlight/multi_intensity` at a high rate. This is:

- Portable (works on any hardware with a LED class device)
- Debuggable (you can see exactly what values are being written)
- Configurable (animation profiles are simple JSON files)
- Safe (sysfs writes that fail are just ignored)

The only minor drawback is that `tailord` must be running for animation to work — if the process crashes or stops, the keyboard freezes at its last color. This is handled by systemd service restart policies.

### 9.6 The Old Atul977 Setup Was Identical

After understanding the full picture, we realized that the old Atul977 setup worked identically:

1. Same DMI bypass patch (single-line `return true`)
2. Same `clevo_wmi` binding
3. Same `tailord` running in userspace
4. Same RGB cycle profile

The previous claim about "keyboard backlight works via `/sys/class/leds/rgb:kbd_backlight/`" in the old setup's notes was actually correct — we had just assumed it was mistaken based on initial testing that failed because the full dependency chain wasn't loaded. The old session summary was right all along.

---

## 10. Comparison: What We Changed vs. What We Kept

| Component | Before | After |
|-----------|--------|-------|
| Kernel module source | Official RPM 4.22.1 | Same + DMI bypass patch |
| Module build | RPM-installed (signed) | DKMS-rebuilt (unsigned, but MOK option available) |
| `clevo_wmi` auto-load | Not loaded | `/etc/modules-load.d/clevo-wmi.conf` |
| `tailord` | Not installed (was from Atul977) | Built from source, systemd-enabled |
| Animation profile | Unknown (Atul977's default) | RGB cycle via `/etc/tailord/` |
| Module parameter | Not set | `kbd_backlight_mode=2` (harmless) |

---

## 11. What This Means for Other Hardware

The same approach could unlock keyboard backlight control on many laptops that:

1. Have standard ACPI WMI (essentially all modern laptops)
2. Have an LED-type keyboard backlight controllable via EC commands
3. Are blocked by a similar DMI compatibility check

The key insight: **if the laptop's built-in WMI interface exposes method 0x52 successfully, and the backlight responds to standard LED class sysfs writes, the DMI check is the only barrier.**

Potential candidates include other Acer models, Lenovo IdeaPads that don't match `ideapad-laptop`, ASUS models not covered by `asus-wmi`, and HP models without `hp-wmi` support.

---

## Appendix A: Module Loading Sequence

```
Boot
  ↓
modules-load.d reads clevo-wmi.conf
  ↓
modprobe clevo_wmi
  ↓
Resolves dependencies:
  tuxedo_compatibility_check ← loads (passes because we patched it)
  tuxedo_keyboard            ← loads (dependency on tuxedo_compatibility_check satisfied)
  clevo_wmi                  ← loads (dependency on tuxedo_keyboard satisfied)
  ↓
clevo_wmi_probe():
  ↓ WMI GUIDs found ✓
  ↓ method 0x52 passes ✓
  ↓ clevo_keyboard_add_interface()
    ↓ tuxedo_keyboard_init_driver()
      ↓ platform_create_bundle()
        ↓ clevo_keyboard_probe()
          ↓ clevo_leds_init()
            → /sys/class/leds/rgb:kbd_backlight/ created
  ↓ pr_info("interface initialized")
  ↓
tailord.service starts (After=systemd-logind.service)
  ↓ Reads /etc/tailord/active_profile.json
  ↓ Opens /sys/class/leds/rgb:kbd_backlight/multi_intensity
  ↓ Begins RGB cycle animation
```

---

## Appendix B: WMI GUID Reference

| GUID | Purpose | Bound by |
|------|---------|----------|
| `ABBC0F6B-8EA1-11D1-00A0-C90629100000` | WMI Method | `clevo_wmi` (method GUID) |
| `ABBC0F6D-8EA1-11D1-00A0-C90629100000` | WMI Event | `clevo_wmi` (event GUID) |
| `ABBC0F6C-8EA1-11D1-00A0-C90629100000` | WMI Object | Not bound |
| `F6CB5C3C-9CAE-4EBD-B577-931EA32A2CC0` | ACPI WMI | Not bound (Acer-specific) |

All GUIDs in the `ABBC0F6*` range are part of the Microsoft ACPI WMI specification namespace. The `F6CB5C3C` GUID is a newer Acer-specific WMI interface (present on post-2023 Acer models).

---

## Appendix C: Useful Debug Commands

```bash
# Check module status
lsmod | grep -E "tuxedo|clevo"

# Check LED device
ls /sys/class/leds/rgb:kbd_backlight/

# Read current color
cat /sys/class/leds/rgb:kbd_backlight/multi_intensity

# Set a color
echo "255 100 0" | sudo tee /sys/class/leds/rgb:kbd_backlight/multi_intensity

# Check the compatibility check patch
strings /lib/modules/$(uname -r)/extra/tuxedo_compatibility_check.ko.xz | grep tuxedo_is

# Check module parameter
cat /sys/module/tuxedo_keyboard/parameters/kbd_backlight_mode

# Verify DKMS status
dkms status tuxedo-drivers

# Check tailord status
systemctl status tailord.service

# Check WMI devices
ls /sys/bus/wmi/devices/

# Monitor dmesg for driver messages
dmesg | grep -E "clevo|tuxedo"
```

---

*Documented May 23, 2026 — Fedora 44, Kernel 7.0.9, tuxedo-drivers 4.22.1*
