# Flashpoint

An open embedded OS platform for ESP32 devices. Boot from SD card, display output, can't brick the device.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  SD card / internal flash                               │
│  ┌─────────────────────────────────────────────────┐   │
│  │  flashpoint.rom  (kernel + header)              │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
         ↑ loaded by
┌─────────────────────────────────────────────────────────┐
│  firmware  (burned once to internal flash)              │
│  ├── Stage 1 chainloader                               │
│  └── Platform HAL  (display, touch, SD, NVS)           │
└─────────────────────────────────────────────────────────┘
         ↑ shared types
┌─────────────────────────────────────────────────────────┐
│  common  (no_std library)                               │
│  ├── ROM header format + validation                     │
│  ├── Platform trait  (hardware abstraction)             │
│  └── Feature flags, ChipId, Event                      │
└─────────────────────────────────────────────────────────┘
```

### Workspace crates

| Crate      | Role |
|------------|------|
| `common`   | Shared types: ROM header, Platform trait, feature flags |
| `firmware` | Device firmware: Stage 1 chainloader + CYD HAL drivers |
| `kernel`   | Hardware-agnostic OS kernel, loaded by Stage 1 at runtime |
| `stage1`   | Chainload logic (validates ROM header, jumps to kernel) |
| `tools`    | `mkrom` CLI: pack and verify `.rom` files |
| `xtask`    | Build orchestration (`cargo xtask <cmd>`) |
| `emulator` | QEMU/Wokwi test binary (ESP-IDF std, logs boot sequence) |

## Quick Start

### Prerequisites

```bash
# Install Rust + ESP Xtensa toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install espup && espup install

# Install flash/build tools
cargo install espflash ldproxy

# Install QEMU (ESP32 fork) — https://github.com/espressif/qemu/releases
# Place qemu-esp-xtensa on your PATH

# Activate the ESP toolchain env (run once per shell)
source scripts/export-esp.sh

# Verify everything is ready
cargo xtask setup
```

### Build & run in the emulator (QEMU)

```bash
cargo xtask emu-run
```

### Build the kernel ROM (for SD card)

```bash
cargo xtask build-boot --platform esp32 --version 0.1.0
# → flashpoint.rom  (copy this to the root of your SD card)
```

### Build and flash device firmware

```bash
# First flash (burns Stage 1 + HAL to internal flash)
cargo xtask flash --port /dev/ttyUSB0 --board esp32-cyd

# With kernel embedded in flash (no SD card needed)
cargo xtask flash --port /dev/ttyUSB0 --board esp32-cyd --embed-boot
```

### All xtask commands

```
cargo xtask setup                     # check deps
cargo xtask build-boot                # compile kernel → flashpoint.rom
cargo xtask build-flash               # compile firmware ELF
cargo xtask build-flash --embed-boot  # firmware + embedded kernel
cargo xtask build-image               # merged flash binary (espflash/QEMU ready)
cargo xtask emu-build                 # build emulator → emulator/flash.bin
cargo xtask emu-run                   # emu-build + launch QEMU
cargo xtask flash --port /dev/ttyUSBx # build + flash to device
```

## Documentation

- [`docs/design.md`](docs/design.md) — full system design and architecture rationale
- [`docs/spec/`](docs/spec/) — ROM format specification
- [`docs/plans/`](docs/plans/) — implementation phase plans

## Supported Hardware

| Board | Status |
|-------|--------|
| ESP32-2432S028R (CYD) | planned (Phase 0) |
| Emulator (QEMU esp32) | working |

## License

- `firmware/`, `stage1/` — AGPL-3.0 (see `LICENSE-FLASH`)
- `kernel/`, apps — MIT (see `LICENSE-BOOT`)
