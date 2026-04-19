# Flashpoint Testing Guide

## Boot Chain Architecture

```
ESP32 ROM Bootloader  (chip-level, immutable — Espressif owns this)
  └─► Flashpoint Firmware / Flash-ROM
        Crate: firmware
        Burned once to ESP32 internal flash. Contains Stage 1 + HAL.
        Build cmd: cargo xtask build-flash --board esp32-cyd
        └─► Flashpoint Software / Boot-ROM   (flashpoint.rom)
              Crate: kernel, packed by xtask
              Loaded at runtime from SD card (or embedded in firmware).
              Contains the Flashpoint OS. Hardware-agnostic.
              Build cmd: cargo xtask build-boot
```

Key rule: **the firmware never changes once flashed**. All OS updates ship
as a new `flashpoint.rom` copied to the SD card.

---

## QEMU Testing (no hardware needed)

All three commands use `cargo xtask` via `cargo run -p xtask --`.

### Full boot in one command
```sh
cargo xtask emu-run
```
This builds the kernel, packs it as `flashpoint.rom`, compiles firmware with
the ROM embedded (board-qemu feature), merges a flash image, and launches QEMU.

### Step-by-step
```sh
# Build the kernel and pack it
cargo xtask build-boot

# Verify the output
cargo xtask verify flashpoint.rom

# Build the QEMU flash image (embeds flashpoint.rom into firmware)
cargo xtask emu-build

# Launch QEMU (uses the image from emu-build)
cargo xtask emu-run
```

### Expected serial output (QEMU)
```
================================
  FLASHPOINT  v0.1.0  [QEMU]
================================
[stage1] header OK — payload at offset 64
[stage1] checksum OK
[stage1] jumping to kernel...
================================
[display] clear
[display] scanline y=0
[display] scanline y=60
[display] scanline y=120
[display] scanline y=180
```

---

## CYD Hardware Testing

**Hardware:** ESP32-2432S028R ("Cheap Yellow Display"), connected via USB.  
**Port:** `/dev/ttyUSB0` (CH340 adapter — auto-resets into flash mode).

### Check the device is reachable
```sh
export PATH="$HOME/.cargo/bin:$PATH"
espflash board-info --port /dev/ttyUSB0
```
Expected: `Chip type: esp32 (revision v3.1)`, `Flash size: 4MB`, `Security features: None`.

### Flash the firmware (Stage 1 + HAL)
```sh
cargo xtask flash --port /dev/ttyUSB0 --board esp32-cyd
```

### Open serial monitor
```sh
cargo xtask monitor --port /dev/ttyUSB0
# Ctrl+] to exit
```

### Expected serial output (no SD card, no embedded ROM — initial state)
```
I (343) firmware: ================================
I (343) firmware:   FLASHPOINT  v0.1.0  [CYD]
I (343) firmware: ================================
I (353) firmware::stage1: [stage1] CYD boot — checking SD card
I (363) firmware::stage1: [stage1] no SD card — checking internal flash
E (363) firmware::stage1: [stage1] no internal boot ROM — halting
```
The device then enters a idle loop (no WDT spam — it yields to FreeRTOS).
This is the expected "no boot ROM" halt state. Verified on CYD rev3.1.

### SD card boot (requires Plan 05 CYD HAL — not yet implemented)
```sh
# Build the kernel
cargo xtask build-boot

# Copy to SD card root (FAT32)
cp flashpoint.rom /path/to/sdcard/

# Eject SD card, insert into CYD, power cycle
# Monitor output:
cargo xtask monitor --port /dev/ttyUSB0
```
Expected:
```
[stage1] CYD boot — checking SD card
[stage1] SD card ready — loading flashpoint.rom
[stage1] SD ROM valid — jumping to 0x3FFB8040
[display] clear
[display] scanline y=0
...
```

### Embedded ROM boot (kernel baked into firmware)
```sh
cargo xtask flash --port /dev/ttyUSB0 --board esp32-cyd --embed-boot
```
Removes the SD card dependency — the kernel is stored in internal flash.

---

## Command Reference

| Command | What it does |
|---------|-------------|
| `cargo xtask setup` | Check all build dependencies |
| `cargo xtask build-boot` | Build kernel → pack as `flashpoint.rom` |
| `cargo xtask build-flash --board esp32-cyd` | Build CYD firmware (Stage 1 + HAL stubs) |
| `cargo xtask build-flash --board esp32-cyd --embed-boot` | Same + embed kernel |
| `cargo xtask flash --port /dev/ttyUSB0` | Build + flash CYD firmware |
| `cargo xtask flash --port /dev/ttyUSB0 --embed-boot` | Build + flash with embedded kernel |
| `cargo xtask monitor --port /dev/ttyUSB0` | Serial monitor (Ctrl+] exits) |
| `cargo xtask emu-build` | Build QEMU flash image |
| `cargo xtask emu-run` | Full QEMU boot (build + run) |
| `cargo xtask verify <file>` | Parse and validate a `.rom` file |
| `cargo xtask pack ...` | Manually pack a binary as `flashpoint.rom` |

---

## Validating a ROM File

```sh
cargo xtask verify flashpoint.rom
```

Output shows magic, platform, API version compatibility, required features,
payload size, and SHA-256 checksum verification.

---

## Running Tests

```sh
# All host tests (common + firmware stage1 logic + xtask rom)
cargo test -p common
cargo test -p firmware --no-default-features   # stage1 logic is board-agnostic
cargo test -p xtask
```

All 28 tests run on the host — no hardware or QEMU needed.
