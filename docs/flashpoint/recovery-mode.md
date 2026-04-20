# Flashpoint Recovery Mode

Recovery mode is entered by holding the **BOOT button (IO0)** while the device powers on or resets. It is also entered automatically when no valid ROM is found in either the SD card or internal flash.

---

## Entry Conditions

| Condition | Behaviour |
|-----------|-----------|
| BOOT button held at reset | Enters recovery immediately, skips ROM search |
| No ROM on SD card AND no ROM in internal flash | Enters recovery after exhausting all boot options |
| SD ROM header invalid | Logs warning, falls through to internal flash, then recovery if that also fails |
| Internal flash ROM header invalid | Logs error, enters recovery |

---

## Display

On devices with a TFT display (`FEAT_DISP_TFT`), recovery renders a **colour-banded menu**:

- Each menu item occupies an equal horizontal band across the full screen width
- The **active (selected) item** is shown at full brightness with **black text**
- **Inactive items** are shown at ~25% brightness with **white text**
- Labels are 8×8 pixel bitmap text, horizontally centred within each band
- Navigation uses `BtnUp` / `BtnDown` events; confirm with `BtnSelect`

On devices without a display, recovery runs a **console-only path**: logs each test result over serial and reboots after 3 seconds.

---

## Menu Items

### Implemented

#### DISPLAY TEST
- Fills the screen with 5 sequential full-screen stripes: red, green, blue, white, black
- Holds each colour for a combined ~2 seconds then returns to the menu
- Purpose: verify display hardware and all colour channels work correctly

#### TOUCH TEST
- Fills screen with a colour corresponding to the last touch event direction
- Up=cyan, Down=red, Left=blue, Right=green, neutral=dark grey
- Auto-exits after 5 seconds or immediately on `BtnSelect`
- Purpose: verify touchscreen axes and event mapping

#### LED TEST
- Cycles the onboard RGB LED through: red → green → blue → yellow → white → off
- Each step held for 400 ms
- Gracefully skips if `led_rgb()` returns `NotSupported`
- Purpose: verify RGB LED wiring and active-low drive

#### REBOOT
- Logs reboot intent, waits 500 ms, calls `platform.reboot()`
- Always the last item in the list

---

### Planned

#### WIFI AP RECOVERY *(requires `FEAT_WIFI`)*
- Only shown on WiFi-capable devices
- Intended to start a soft AP named `flashpoint-recovery` with an open HTTP file server
- Allows drag-and-drop ROM upload from a browser without needing a serial connection or SD card
- Status: stub in place, logs "not yet implemented"

#### SD FORMAT / REPAIR
- Detect and optionally reformat the SD card if the filesystem is corrupt
- Requires `FEAT_SD` capability flag (not yet defined)

#### NVS WIPE
- Clear all NVS namespaces (saved settings, WiFi credentials, user data)
- Requires a confirmation step before executing

#### FIRMWARE UPDATE (OTA)
- Flash a new `firmware.bin` from SD card or WiFi AP
- Requires bootloader OTA partition support

#### INFO / DIAGNOSTICS
- Display chip ID, MAC address, flash size, free heap, SDK version
- Useful for support and debugging without a serial monitor

---

## Colour Palette

| Item | Active (RGB565) | Inactive |
|------|----------------|----------|
| DISPLAY TEST | `0xF81F` magenta | dimmed magenta |
| TOUCH TEST | `0x07FF` cyan | dimmed cyan |
| LED TEST | `0xFFE0` yellow | dimmed yellow |
| WIFI AP | `0x001F` blue | dimmed blue |
| REBOOT | `0xF800` red | dimmed red |

Dimming is applied by right-shifting each channel 2 bits: `r>>2, g>>2, b>>2`.

---

## Architecture Notes

- `recovery_main()` is in `common/src/lib.rs` — fully hardware-agnostic
- Platform capabilities are queried via `platform.features()` bitmask at runtime; menu items are added/hidden accordingly
- All drawing goes through the `Platform` trait (`display_flush`, `display_width`, `display_height`) — no HAL imports in common
- Text rendering uses the built-in `font_glyph()` 8×8 bitmap font (no external font crate required in common)
- The glyph row order is inverted (`[7 - char_row]`) to correct for the CYD display's physical y-axis orientation
