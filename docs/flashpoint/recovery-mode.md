# Flashpoint Recovery Mode

Recovery mode is entered by holding the **BOOT button (IO0)** while the device powers on or resets. It is also entered automatically when no valid ROM is found in either the SD card or internal flash.

**UART console access is always active** in recovery mode (both display-equipped and console-only paths) unless explicitly disabled via the `no-uart-recovery` build feature.

---

## Architecture

- **Core recovery logic** lives in `common/src/lib.rs` — fully hardware-agnostic.
- **HAL-specific activation**: how recovery mode is entered (CYD: BOOT button IO0, QEMU: no hardware trigger) is defined in `firmware/src/stage1.rs` per-board.
- **HAL-specific control**: `uart_poll_byte()` on the Platform trait provides serial input; each HAL implements it for its UART hardware.
- **Unified input**: `poll_recovery_input()` checks both hardware events (touch/buttons via `poll_event()`) and UART bytes, so all recovery options work via serial regardless of display availability.

---

## Entry Conditions

| Condition | Behaviour |
|-----------|-----------|
| BOOT button held at reset | Enters recovery immediately, skips ROM search |
| No ROM on SD card AND no ROM in internal flash | Enters recovery after exhausting all boot options |
| SD ROM header invalid | Logs warning, falls through to internal flash, then recovery if that also fails |
| Internal flash ROM header invalid | Logs error, enters recovery |

---

## UART Console Access

All recovery mode paths retain full UART serial interaction. A user connected via serial monitor can:

- **Navigate**: `w`/`k` = up, `s`/`j` = down
- **Select**: `Enter`, `Space`
- **Direct select**: number keys `1`-`9` to run a menu item directly
- **Back**: `q` or `ESC`

The menu state is logged over UART on every selection change, so a serial user always sees the current options and active selection.

To disable UART recovery interaction at build time, enable the `no-uart-recovery` feature on the `common` crate. When disabled:
- Display path: only touch/button input accepted
- Console path: runs basic hardware tests automatically and reboots after 3 seconds

---

## Display

On devices with a TFT display (`FEAT_DISP_TFT`), recovery renders a **colour-banded menu**:

- Each menu item occupies an equal horizontal band across the full screen width
- The **active (selected) item** is shown at full brightness with **black text**
- **Inactive items** are shown at ~25% brightness with **white text**
- Labels are 8×8 pixel bitmap text, horizontally centred within each band
- Navigation uses `BtnUp` / `BtnDown` events; confirm with `BtnSelect`
- UART commands work simultaneously alongside touch/button input

On devices without a display, recovery runs an **interactive UART menu**: logs menu items with numbers over serial and accepts UART commands for navigation and selection.

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

### Planned / Stub

#### WIFI AP RECOVERY *(requires `FEAT_WIFI`)*
- Only shown on WiFi-capable devices
- Intended to start a soft AP named `flashpoint-recovery` with an open HTTP file server
- Allows drag-and-drop ROM upload from a browser without needing a serial connection or SD card
- Status: stub in place, logs "not yet implemented"

#### USB MOUNT SD *(requires `FEAT_USB_OTG`)*
- Only shown on devices with USB OTG support
- Exposes the SD card as a USB mass storage device so the user can transfer ROMs to/from the SD card without removing it physically
- Boot-ROMs may implement their own take on SD/USB file transfer via host API instead of relying on this recovery menu implementation
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
| USB MOUNT SD | `0x07E0` green | dimmed green |
| REBOOT | `0xF800` red | dimmed red |

Dimming is applied by right-shifting each channel 2 bits: `r>>2, g>>2, b>>2`.

---

## Build Features

| Feature | Effect |
|---------|--------|
| *(default)* | UART console active in all recovery paths |
| `no-uart-recovery` | Disables UART input in recovery; display path is touch-only, console path auto-tests and reboots |
