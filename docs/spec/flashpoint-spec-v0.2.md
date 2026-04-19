# ⚡ FLASHPOINT
## Platform Specification v0.1-draft
### Design Document & Agent Handoff

| Field | Value |
|---|---|
| Status | Active — Implementation in progress |
| Revision | 0.2 |
| Date | 2026-04-19 |
| Authors | Merith + Claude (Anthropic) |
| Target HW | ESP32 / ESP32-S3 (reference); RP2040 (planned port) |
| License | TBD — open spec intended |

---

## Table of Contents

1. [Overview](#1-overview)
2. [System Architecture](#2-system-architecture)
3. [Boot Sequence](#3-boot-sequence)
4. [boot-rom Binary Format](#4-boot-rom-binary-format)
5. [Hardware Abstraction Layer](#5-hardware-abstraction-layer)
6. [card-ram Paging Layer](#6-card-ram-paging-layer)
7. [KernelFS](#7-kernelfs)
8. [Application Model](#8-application-model)
9. [Host API Surface](#9-host-api-surface)
10. [Shell UI](#10-shell-ui)
11. [Development Guide](#11-development-guide)
12. [Implementation Roadmap](#12-implementation-roadmap)
13. [Agent Handoff](#13-agent-handoff)
14. [Glossary](#14-glossary)

---

## 1. Overview

Flashpoint is an open embedded platform specification targeting microcontrollers in the ESP32 family (and compatible hardware). It defines a chainloadable OS binary format, a hardware abstraction layer (HAL) contract, a kernel-level paging system, a dual-runtime application model (WebAssembly + Lua), and a standardised app packaging format.

The name derives from the temperature at which a substance ignites — a fitting metaphor for a system that boots from a cold start into a fully running OS, and can deliver that OS from external media at any time.

### 1.1 Design Philosophy

- The `flash-rom` is burned once and ideally never reflashed. It owns hardware init, all device drivers, and the HAL abstraction layer.
- The `boot-rom` (the OS) is fully replaceable at runtime by dropping `flashpoint.rom` onto the SD card.
- Hardware is abstracted behind a single `Platform` trait — porting to new hardware means implementing that trait in the `flash-rom` only. The `boot-rom` never touches hardware directly.
- Apps run in sandboxed WASM or Lua runtimes and cannot corrupt the kernel.
- Security-sensitive data never leaves internal flash storage.
- The platform spec is independent of any single device — community ports are first-class.
- A `flash-rom` is valid without an embedded `boot-rom`. It will wait for an SD card with `flashpoint.rom`.
- A `boot-rom` declares what hardware features it requires. The `flash-rom` declares what features it provides. The loader enforces compatibility before executing any boot-rom code.

### 1.2 Terminology

| Term | Definition |
|---|---|
| `flash-rom` | Firmware burned to internal ESP32 flash. Contains Stage 1 loader, all hardware drivers, the HAL implementation, and optionally an embedded `boot-rom`. Burned once; ideally never reflashed. |
| `boot-rom` | The OS binary. Contains kernel, shell, and runtimes. Runs entirely on the HAL exposed by `flash-rom`. Distributed as `flashpoint.rom` on SD card, or embedded inside a `flash-rom`. The same binary either way. |
| `flashpoint.rom` | A `boot-rom` packaged with a Flashpoint header. Placed on the SD card's FAT32 partition to trigger chainloading. |
| `embedded boot-rom` | A `boot-rom` compiled directly into the `flash-rom` binary. Provides a guaranteed fallback when no SD card is present. |
| `card-ram` | The raw MMC paging partition on the SD card. Used for virtual memory and hibernate state. |
| `KernelFS` | Internal NVS-backed secure key-value store on chip flash. Holds credentials and core extensions. |
| `Stage 1` | Minimal loader at the start of `flash-rom`. Checks SD card for `flashpoint.rom`, falls back to embedded boot-rom. Uses compile-time constants for all internal offsets. |
| `Stage 2` | The `boot-rom` itself. Kernel, shell, runtimes — everything above the HAL. |
| `HAL` | Hardware Abstraction Layer. The `Platform` Rust trait, implemented inside `flash-rom`. The `boot-rom` calls into it but never implements it. |
| `feature flags` | A bitmask in the `boot-rom` header declaring what hardware capabilities it requires. The loader checks these against the `flash-rom`'s published capability bitmask before executing. |
| `App` | A user application. Consists of `app.ini` + `app.wasm` or `app.lua` + assets in `/apps/<n>/`. |
| `tile` | The passive display descriptor for an app shown in the launcher. Shell-rendered only. |

---

## 2. System Architecture

### 2.1 Storage Tiers

| Domain | Medium | Contents | Trust Level |
|---|---|---|---|
| Internal Flash | ESP32 flash (4–16 MB) | Stage 1, embedded `boot-rom` (optional), KernelFS | Trusted — kernel domain |
| SD Card FAT32 | SD card FAT32 partition | `/apps/`, `sdboot.rom`, user data | Untrusted — user domain |
| SD Card MMC | SD card raw sectors (1 GB) | `card-ram` paging, hibernate state | Untrusted — paged data |

### 2.2 Internal Flash Layout

Internal flash is managed via the ESP-IDF partition table (`partitions_cyd.csv`). Stage 1 locates partitions at runtime via `esp_partition_find()` — no hardcoded offsets. Slot sizes are tracked in NVS.

```
CYD (4 MB) — partitions_cyd.csv
┌──────────────────────────────────────────────────┐
│ nvs              data/nvs       20 KB  (0x5000)  │  ← ESP-IDF NVS
│ phy_init         data/phy        4 KB  (0x1000)  │  ← RF calibration (written once)
│ app0             app/ota_0    1.5 MB (0x180000)  │  ← flash-rom (Stage 1 + HAL)
│ flashpoint_nvs   data/nvs     512 KB  (0x80000)  │  ← Flashpoint KernelFS
│ flashpoint_rom   data/0x40      1 MB (0x100000)  │  ← embedded boot-rom slot
│ flashpoint_user  data/0x41      1 MB (0x100000)  │  ← SD-updated user ROM slot
└──────────────────────────────────────────────────┘
```

**Slot size tracking** — Stage 1 reads/writes these NVS keys before loading:

```
flashpoint.nvs/rom-embedded-size  = u32  (0 = slot empty)
flashpoint.nvs/rom-user-size      = u32  (0 = slot empty)
```

Compile-time constants (`BOOTROM_OFFSET`, `BOOTROM_SIZE`, `NVS_OFFSET`) still exist for backward compatibility with the build system but Stage 1 prefers the partition API.

### 2.3 SD Card Layout

```
SD Card (MBR partition table)
├── Partition 1  type=0xDA  offset=0           size=1 GB     raw MMC (card-ram)
│     ├── Sector 0         page table checkpoint + hibernate header
│     ├── Sectors 1–N      page frame backing store
│     └── Sectors N+1…     hibernate framebuffer region
└── Partition 2  type=0x0B  offset=2,097,152   size=remainder  FAT32
      ├── flashpoint.rom    ← triggers chainload if present and valid
      ├── /apps/            ← user applications
      └── /boot/            ← reserved (future multi-platform roms)
```

### 2.4 Software Layer Stack

```
╔══════════════════════════════════════════════╗
║  boot-rom  (flashpoint.rom / embedded)       ║
║                                              ║
║  User Applications                           ║
║  WASM (wasm3 runtime)  |  Lua (Lua 5.x VM)  ║
║──────────────────────────────────────────────║
║  Shell                                       ║
║  Status bars | Battery | App grid | Dropdown ║
║──────────────────────────────────────────────║
║  Kernel                                      ║
║  card-ram paging | FatFS | NVS | Event loop  ║
╠══════════════════════════════════════════════╣
║  flash-rom  (burned once to device)          ║
║                                              ║
║  HAL  (Platform trait implementation)        ║
║  display | input | storage | system          ║
║──────────────────────────────────────────────║
║  Hardware Drivers                            ║
║  ILI9341 | XPT2046 | SDMMC | NVS | …        ║
║──────────────────────────────────────────────║
║  Stage 1 Loader                              ║
║  SD probe → header validate → chainload      ║
╠══════════════════════════════════════════════╣
║  Hardware                                    ║
║  ESP32 / ESP32-S3 / RP2040 / …              ║
╚══════════════════════════════════════════════╝
```

The boundary between `flash-rom` and `boot-rom` is the `Platform` trait. The `flash-rom` implements it; the `boot-rom` calls it. This means a `boot-rom` binary is hardware-agnostic — it runs on any device that has a compatible `flash-rom` and meets the declared feature requirements.

---

## 3. Boot Sequence

### 3.1 Stage 1 — Chainload Logic (5-Step Boot Priority)

Stage 1 is burned once and ideally never updated. Its sole job is to decide what to run.

```
Power On
│
├── ESP32 ROM Bootloader (chip-level, immutable)
│
├── Stage 1
│     │
│     ├── Step 1: Recovery key held at power-on?
│     │     (CYD: touch held during boot, detected via GPIO before SPI init)
│     │     YES → enter Recovery Menu
│     │
│     ├── Step 2: SD card present AND flashpoint.rom found?
│     │     a. Validate header (magic, CRC32, platform, features, payload_len)
│     │     b. Compare SD header vs flashpoint_user slot (CRC32 + version)
│     │           Same?      → skip write, proceed to Step 3
│     │           Different, fits? → write to flashpoint_user, update NVS size
│     │           Different, too large? → log warning, offer override in Recovery Menu
│     │     c. Continue to Step 3 (user slot now has the SD ROM)
│     │
│     ├── Step 3: flashpoint_user slot valid?
│     │     (written by SD boot or manual flash)
│     │     YES → validate header, dispatch by PayloadType:
│     │             Native  → XIP jump (code runs from flash)
│     │             Wasm32  → load into heap, run via wasm3
│     │             Luac54  → load into heap, run via Lua 5.4
│     │
│     ├── Step 4: flashpoint_rom slot valid?
│     │     (embedded at build time via --embed-boot)
│     │     YES → same dispatch as Step 3
│     │
│     └── Step 5: Recovery Menu
│           (no bootable ROM anywhere — device is not bricked, just empty)
│
└── Stage 2 (boot-rom) begins
```

**Size constraint check** — before any load, Stage 1 enforces per-type limits:

```
Native  → must fit in flashpoint_user partition (1 MB on CYD)
Wasm32  → must fit in wasm_arena_limit()         (256 KB on CYD, no PSRAM)
Luac54  → must fit in lua_heap_limit()            (64 KB on CYD)
```

Oversized ROMs are rejected and Stage 1 falls through to the next step.

### 3.2 Stage 2 — Kernel Init

```
Stage 2 entry
├── PSRAM init
├── NVS init at NVS_OFFSET (compile-time constant, always correct)
├── Display driver init → splash screen
├── Button / input init
├── Battery ADC init
├── card-ram paging layer init
│     └── Read MMC sector 0
│           ├── Hibernate signature present? → RESUME PATH
│           └── No signature                → FRESH BOOT PATH
│
├── RESUME PATH
│     ├── Restore display framebuffer from MMC
│     ├── Restore shell tile cache from MMC
│     ├── Re-init runtime for last active app
│     ├── Call app init()  ← app restores own state from data/
│     └── Resume event loop
│
└── FRESH BOOT PATH
      ├── Scan /apps/ → parse each app.ini
      ├── Build shell tile cache
      └── Render launcher → enter event loop
```

---

## 4. boot-rom Binary Format

The `boot-rom` format is the core interchange format of Flashpoint. Any conforming Stage 1 must parse and validate this header before chainloading or writing to internal flash.

### 4.1 Header Layout (v2 — spec 0.2)

The header is exactly **64 bytes**. The payload begins immediately after at byte `header_size`. The `header_size` field and `FLPE` end magic make the format forward-compatible.

| Field | Offset | Type | Size | Value / Notes |
|---|---|---|---|---|
| `magic` | `0x00` | `u8[4]` | 4 | ASCII `FLPT` |
| `platform` | `0x04` | `u8` | 1 | `0x01`=ESP32  `0x02`=ESP32-S3  `0x03`=RP2040  `0xFF`=any |
| `rom_version` | `0x05` | `u8[3]` | 3 | Semantic version `[major, minor, patch]` |
| `built_against` | `0x08` | `u32 LE` | 4 | Flashpoint API version this ROM targets |
| `flags` | `0x0C` | `u16 LE` | 2 | Bit 0: compressed payload. Bits 1–15: reserved. |
| `required_features` | `0x0E` | `u64 LE` | 8 | Hardware bitmask (see §4.4) |
| `payload_len` | `0x16` | `u32 LE` | 4 | Length of payload in bytes |
| `crc32` | `0x1A` | `u32 LE` | 4 | CRC-32/ISO-HDLC of payload bytes |
| `payload_type` | `0x1E` | `u8` | 1 | `0x00`=native  `0x01`=wasm32  `0x02`=luac54 |
| `rom_id` | `0x1F` | `u8[24]` | 24 | Null-terminated ASCII namespace, max 23 chars (e.g. `com.flashpoint.shell`) |
| `compat_platforms` | `0x37` | `u8[3]` | 3 | Up to 3 additional platform bytes. `0x00`=end, `0xFF`=any wildcard. |
| `header_size` | `0x3A` | `u16 LE` | 2 | `0x0040` (64). Payload starts at this offset. |
| `header_end` | `0x3C` | `u8[4]` | 4 | ASCII `FLPE` — mirrors `FLPT` at start. |
| **Total** | | | **64 bytes** | Payload begins at byte 64 |

**Changes from v1:** SHA-256 (32 bytes) → CRC32 (4 bytes), freeing space for `payload_type`, `rom_id`, and `compat_platforms`. End byte `0xFE` → 4-byte `FLPE` magic.

**CRC32 rationale:** Guards against corruption only (not a security hash). Uses `CRC-32/ISO-HDLC` (`esp_rom_crc32_le()` on hardware, pure-Rust `crc` crate in xtask and host tests).

**Platform matching:** Stage 1 accepts a ROM if `our_platform` matches the primary `platform` byte, any non-zero `compat_platforms` entry, or any entry is `0xFF` (wildcard).

**ROM ID / NVS namespace:** For native and WASM payloads, NVS keys are stored as `{rom_id}/{key}`. Lua payloads are denied NVS access (flat-file SD filesystem only). Stage 1 passes the ROM ID to the Platform trait before jumping.

Stage 1 validates all fields before executing anything. Any failure causes a fallback to the next boot step. **The device cannot be bricked by a malformed `flashpoint.rom`.**

### 4.2 build.rs Logic

```rust
fn main() {
    let bootrom_path = std::env::var("BOOTROM_BIN").ok();
    let stage1_end: u32 = 0x10000;

    let (bootrom_offset, bootrom_size, nvs_offset) = match bootrom_path {
        Some(path) => {
            let size = std::fs::metadata(&path).unwrap().len() as u32;
            let aligned = align_up(size, 0x1000);
            (stage1_end, aligned, stage1_end + aligned)
        }
        None => (0u32, 0u32, stage1_end),
    };

    println!("cargo:rustc-env=BOOTROM_OFFSET={}", bootrom_offset);
    println!("cargo:rustc-env=BOOTROM_SIZE={}", bootrom_size);
    println!("cargo:rustc-env=NVS_OFFSET={}", nvs_offset);
}
```

### 4.3 Multi-Platform ROMs

`platform = 0xFF` (`PLATFORM_ANY`) is the wildcard — Stage 1 accepts the ROM on any device. For richer multi-platform targeting, the `compat_platforms` array (bytes `0x37–0x39`) lists up to three additional accepted platform bytes; `0xFF` in any compat slot also acts as a wildcard.

### 4.4 Feature Flags

The `required_features` field in the header is a `u64` bitmask grouped by hardware category. Stage 1 enforces `(device_features & required) == required` before any ROM code runs.

| Bits | Constant | Meaning |
|------|----------|---------|
| 0 | `FEAT_WIFI` | WiFi hardware present |
| 1 | `FEAT_BLE` | Bluetooth LE present |
| 2 | `FEAT_USB_OTG` | USB OTG present |
| 8 | `FEAT_DISP_TFT` | TFT / LCD display present |
| 9 | `FEAT_DISP_EINK` | eInk display present |
| 16 | `FEAT_INPUT_TOUCH` | Touch input present |
| 17 | `FEAT_INPUT_BUTTONS` | Physical directional buttons present |
| 24 | `FEAT_PSRAM` | External PSRAM / SPIRAM available |
| 25 | `FEAT_BATTERY` | Battery / ADC monitoring present |

```rust
// firmware/src/main.rs — CYD declaration
pub const DEVICE_FEATURES: u64 =
    FEAT_DISP_TFT | FEAT_INPUT_TOUCH;
    // No PSRAM, no WiFi, no battery on base CYD
```

A ROM requiring `FEAT_PSRAM` is rejected on the base CYD but accepted on Xteink X4. The error surfaces before any ROM code executes.

---

## 5. Hardware Abstraction Layer

The HAL lives entirely inside the `flash-rom`. The `boot-rom` contains zero hardware-specific code — it only calls the `Platform` trait. A port to new hardware means writing a new `flash-rom` with a new `Platform` implementation. The `boot-rom` binary is portable across all compliant devices.

The implementation strategy is: use an existing crate or C library wherever one exists and is fit for purpose. Where none exists, or where available options are inadequate, write the minimum necessary driver code. The `Platform` trait is the contract — how it is satisfied behind the scenes is an implementation detail of each `flash-rom` port.

### 5.1 Platform Trait

```rust
pub trait Platform {
    // Storage
    fn sd_read_sectors(&self, start: u32, buf: &mut [u8]) -> Result<()>;
    fn sd_write_sectors(&self, start: u32, buf: &[u8])    -> Result<()>;
    fn sd_sector_count(&self) -> u32;

    fn nvs_read(&self, ns: &str, key: &str)              -> Result<Vec<u8>>;
    fn nvs_write(&self, ns: &str, key: &str, val: &[u8]) -> Result<()>;
    fn nvs_delete(&self, ns: &str, key: &str)            -> Result<()>;

    // Display
    fn display_flush(&self, buf: &FrameBuffer) -> Result<()>;
    fn display_clear(&self)                    -> Result<()>;
    fn display_width(&self)  -> u16;
    fn display_height(&self) -> u16;

    // Input
    fn poll_event(&self) -> Option<Event>;

    // System
    fn battery_percent(&self) -> u8;
    fn chip_id(&self)         -> ChipId;
    fn reboot(&self)          -> !;
    fn sleep_ms(&self, ms: u32);
}
```

### 5.2 Event Enum

```rust
pub enum Event {
    BtnUp, BtnDown, BtnLeft, BtnRight,
    BtnSelect, BtnBack,
    BatteryLow,        // kernel emits at configurable threshold
    HibernateWarning,  // kernel emits before forced hibernate
}
```

### 5.3 Reference Implementations

| Platform | Status | Display | Input |
|---|---|---|---|
| ESP32 (WROOM-32) | Reference — CYD dev board | ILI9341 TFT via SPI | Resistive touch → 4 button zones |
| ESP32-S3 | Reference — Xteink X4 target | eInk via SPI | Physical rocker buttons |
| RP2040 | Planned community port | TBD | TBD |

---

## 6. card-ram Paging Layer

`card-ram` provides virtual memory semantics on hardware without an MMU. The kernel manages a page table in SRAM, a frame pool in PSRAM, and a backing store in the raw MMC partition.

### 6.1 Architecture

```
┌──────────────────┐
│   Page Table     │  ← SRAM (small metadata only)
└────────┬─────────┘
         │
┌────────▼─────────┐
│  Frame Pool      │  ← PSRAM (resident working set)
│  N × PAGE_SIZE   │
└────────┬─────────┘
         │  evict / load
┌────────▼─────────┐
│  MMC Partition   │  ← SD raw sectors
│  (card-ram 1 GB) │
└──────────────────┘
```

### 6.2 Page Table Entry

```c
typedef struct {
    uint32_t sector;     // absolute sector in MMC partition
    void*    frame;      // pointer into frame pool, NULL if evicted
    bool     dirty;      // modified since last write-back?
    uint32_t last_used;  // tick counter for LRU eviction
} page_entry_t;
```

### 6.3 Configuration Constants

| Constant | Recommended Value | Notes |
|---|---|---|
| `PAGE_SIZE` | 4096 bytes | Aligns to flash erase blocks |
| `FRAME_POOL_COUNT` | 16 | 64 KB PSRAM resident. Set to 4 on CYD (no PSRAM). |
| `MMC_START_SECTOR` | 0 | Relative to MMC partition 1 start |
| `MMC_SECTOR_COUNT` | 2,097,152 | 1 GB ÷ 512 bytes |
| `PAGE_TABLE_SECTOR` | 0 | First MMC sector — checkpoint + hibernate header |

### 6.4 Hibernate Procedure

```
Trigger (low battery / user request / graceful app close):
1. Call app shutdown() → app serialises state to /apps/<n>/data/
2. Flush all dirty frames to backing sectors
3. Write page table + hibernate magic to MMC sector 0
4. Write display framebuffer to reserved MMC region
5. Write shell tile cache to MMC
6. Flush FatFS
7. Power off / deep sleep

Resume detection (Stage 2 init):
  Read MMC sector 0
  ├── hibernate magic present → RESUME PATH
  └── absent / corrupt        → FRESH BOOT PATH
```

---

## 7. KernelFS

KernelFS is the internal-flash-only secure storage layer, implemented on top of ESP-IDF NVS. NVS is initialised at `NVS_OFFSET` — a compile-time constant that is always correct regardless of whether a `boot-rom` is embedded. No app ever receives a raw file handle into KernelFS.

### 7.1 Namespace Layout

| NVS Namespace | Contents | Writable By |
|---|---|---|
| `sys` | Device ID, platform version, boot counter | Kernel only |
| `wifi` | SSID, PSK, last IP | Kernel wifi extension only |
| `ext` | Bundled Lua extension metadata | Kernel only |
| `app_<n>` | Per-app secure namespace (credentials, tokens) | That app only, via helper API |

### 7.2 Secure Storage API

Apps declare `secure = true` in `app.ini`. Kernel creates `app_<n>` namespace on first launch.

**WASM imports:**
```wasm
(import "env" "secure_read"   (func (param i32 i32 i32) (result i32)))
  ;; key_ptr, key_len, out_buf_ptr → bytes_written
(import "env" "secure_write"  (func (param i32 i32 i32 i32)))
  ;; key_ptr, key_len, val_ptr, val_len
(import "env" "secure_delete" (func (param i32 i32)))
  ;; key_ptr, key_len
```

**Lua globals:**
```lua
secure.write("api_key", "abc123")
local key = secure.read("api_key")  -- returns string or nil
secure.delete("api_key")
```

> **Security:** An app named `weather` receives namespace `app_weather` only. It cannot read, write, or enumerate any other namespace. The `wifi` namespace is never exposed to apps — only the bundled wifi Lua extension can initiate connections.

### 7.3 Bundled Lua Extensions

Kernel-provided libraries stored in internal flash, available to any Lua app via `require`:

```lua
local wifi   = require("wifi")
local json   = require("json")
local crypto = require("crypto")

wifi.connect()  -- reads credentials from KernelFS internally, never exposes them
```

---

## 8. Application Model

### 8.1 Directory Structure

```
/apps/
  <appname>/
    app.ini       ← manifest ([app] immutable, [tile] mutable)
    app.wasm      ← if type=wasm
    app.lua       ← if type=lua
    icon.bmp      ← launcher icon (scaled to fit, letterboxed, no stretch)
    data/         ← app private read-write sandbox
```

### 8.2 app.ini Format

```ini
[app]
; Immutable — read once at load, kernel ignores runtime writes
name   = Weather
type   = wasm        ; wasm | lua
entry  = app.wasm
secure = false       ; true = request secure namespace

[tile]
; Mutable — updated in memory via tile helper API
; Persisted to disk only if app calls tile.save() / tile_save()
text   = Loading...  ; subtitle shown under icon
color  = white       ; white | black | invert
font   = small       ; small | medium | large
icon   = icon.bmp
badge  =             ; integer or empty (no badge)
```

### 8.3 App Lifecycle

```
Launch
├── Read app.ini → determine type
├── Init runtime  (wasm3 OR Lua VM — never both simultaneously)
├── Load entry binary from SD into PSRAM via card-ram
├── Inject host API
└── Call init()

Running
└── Kernel calls on_event(evt) on each input event
    App owns display API while foregrounded

Shutdown / App Switch
├── Call shutdown()
│     App saves state to data/
│     App updates tile via helpers if needed
├── Flush tile dirty flag (write app.ini [tile] if tile.save() called)
├── Teardown runtime
├── Free PSRAM frames
└── Return to shell
```

### 8.4 Required Entry Points

| Function | WASM export | Lua global | Called when |
|---|---|---|---|
| `init` | `(export "init" (func))` | `function init() end` | App launches or resumes from hibernate |
| `shutdown` | `(export "shutdown" (func))` | `function shutdown() end` | App closed or system hibernating |
| `on_event` | `(export "on_event" (func (param i32)))` | `function on_event(evt) end` | Input event while foregrounded |

---

## 9. Host API Surface

WASM apps use imports from the `env` module. Lua apps use injected globals. Both bind to the same kernel functions.

### 9.1 Filesystem API

All paths are sandboxed to `/apps/<appname>/data/`. Apps cannot traverse outside this directory.

| Function | Parameters | Returns | Notes |
|---|---|---|---|
| `fs_open` | `path_ptr, path_len, mode` | `fd: i32` | mode: 0=read 1=write 2=append |
| `fs_read` | `fd, buf_ptr, buf_len` | `bytes: i32` | -1 on error |
| `fs_write` | `fd, buf_ptr, buf_len` | `bytes: i32` | -1 on error |
| `fs_close` | `fd` | void | |
| `fs_exists` | `path_ptr, path_len` | `i32` | 1=exists 0=not found |
| `fs_delete` | `path_ptr, path_len` | void | |

### 9.2 Display API

Apps receive display access only while foregrounded. Kernel revokes access on shutdown.

| Function | Parameters | Notes |
|---|---|---|
| `draw_bitmap` | `x, y, w, h, ptr` | Blit 1-bit or greyscale bitmap from WASM linear memory |
| `draw_text` | `x, y, ptr, len` | Render text at pixel coords using current font |
| `draw_rect` | `x, y, w, h, fill` | fill: 0=white 1=black 2=invert |
| `display_flush` | — | Commit framebuffer. eInk: triggers partial or full refresh. |
| `display_clear` | — | Fill framebuffer white |
| `set_font` | `font_id` | 0=small 1=medium 2=large |

### 9.3 Tile Helper API

| Function | Parameters | Notes |
|---|---|---|
| `tile_set_text` | `ptr, len` | Sets `[tile] text` in memory |
| `tile_set_badge` | `value: i32` | -1 clears badge. 0–99 shown as number. |
| `tile_set_color` | `value: i32` | 0=white 1=black 2=invert |
| `tile_save` | — | Writes current tile state to `app.ini` on SD. Persists across reboots. |

**Lua equivalents:** `tile.set_text(s)`, `tile.set_badge(n)`, `tile.set_color(s)`, `tile.save()`

### 9.4 Input Events

```lua
function on_event(evt)
  if evt == EVT_BTN_UP             then end
  if evt == EVT_BTN_DOWN           then end
  if evt == EVT_BTN_LEFT           then end
  if evt == EVT_BTN_RIGHT          then end
  if evt == EVT_BTN_SELECT         then end
  if evt == EVT_BTN_BACK           then end
  if evt == EVT_BATTERY_LOW        then
    -- save state before forced hibernate
  end
  if evt == EVT_HIBERNATE_WARNING  then
    -- last chance to act before power off
  end
end
```

---

## 10. Shell UI

### 10.1 Layout

```
┌──────────────────────────────────────────────┐
│ [FLASH 23%]      12:34      [SD 67%]         │  ← status bar
├──────────────────┬───────────────────────────┤
│ ██████░░░ 72%    │ ▼ System                  │  ← battery bar + dropdown
├──────────────────┴───────────────────────────┤
│                                              │
│  [icon]  [icon]  [icon]  [icon]  [icon]      │
│  label   label   label   label   label       │
│                                              │
│  [icon]  [icon]  [icon]  [icon]  [icon]      │
│  label   label   label   label   label       │
│                                              │
└──────────────────────────────────────────────┘
```

### 10.2 Status Bar

| Region | Content | Update Frequency |
|---|---|---|
| Left | Internal flash usage % | On change |
| Centre | System time (HH:MM) | Every minute |
| Right | SD card usage % | On change |

### 10.3 System Dropdown

- Display refresh (full eInk clear, removes ghosting)
- Sleep / Hibernate
- Storage info (flash %, SD %, card-ram %)
- App manager (view active app, force close)
- WiFi settings (connect, forget network)
- About (platform version, spec version, chip ID)

### 10.4 Tile Rendering Contract

The shell owns all rendering. Apps in the background have zero display access. Tile data is pulled from the shell's in-memory cache, populated from `app.ini` at scan time and updated via tile helpers at runtime.

| Tile Element | Source | Constraints |
|---|---|---|
| Icon | `icon.bmp` | Scaled to fit tile space, letterboxed. 1-bit or greyscale BMP. |
| Label | `[tile] text` | Single line, truncated with ellipsis if overflowing. |
| Badge | `[tile] badge` | Integer 0–99 overlaid top-right of icon. Empty = hidden. |
| Color | `[tile] color` | Affects label rendering: white / black / invert. |

---

## 11. Development Guide

### 11.1 Toolchain

| Tool | Purpose | Install |
|---|---|---|
| Rust + espup | Kernel / Stage 1 compilation (Xtensa target) | `cargo install espup && espup install` |
| espflash | Flash binary to device | `cargo install espflash` |
| esp-idf-sys | ESP-IDF Rust bindings (auto-downloads IDF) | Cargo dependency |
| wasm3 | WASM interpreter C library | Git submodule, linked via `cc` crate |
| Lua 5.4 | Lua VM C library | Git submodule, linked via `cc` crate |

### 11.2 Repository Structure

```
flashpoint/
├── flashpoint-common/    shared types: header struct, feature flags, ChipId, Event
│     └── src/lib.rs
├── stage1/               minimal chainload loader (Rust, no_std) — part of flash-rom
│     ├── build.rs        generates BOOTROM_OFFSET / BOOTROM_SIZE / NVS_OFFSET
│     └── src/main.rs
├── flash-rom/            device firmware burned once — drivers, HAL, Stage 1
│     ├── build.rs        orchestrates stage1 build, sets BOOTROM_BIN if embed-bootrom feature set
│     └── src/
│           ├── main.rs
│           ├── capabilities.rs   DEVICE_FEATURES bitmask (generated by build.rs)
│           └── hal/
│                 ├── mod.rs          Platform trait definition
│                 ├── esp32_cyd.rs    CYD ILI9341 + XPT2046 + SDMMC + NVS
│                 └── esp32s3_xteink.rs  Xteink X4 eInk + buttons (Phase 4)
├── boot-rom/             OS kernel — hardware-agnostic, calls Platform trait only
│     └── src/
│           ├── main.rs
│           ├── kernel/   card-ram paging | FatFS | NVS | event loop
│           ├── shell/    status bar | battery | app grid | dropdown
│           └── runtime/  wasm3 + lua integration + host API
├── tools/
│     └── src/mkrom.rs    CLI: wraps payload with Flashpoint header → flashpoint.rom
├── xtask/
│     └── src/main.rs     Build orchestration: build-flash | build-rom | flash
└── spec/
      └── flashpoint-spec-v0.2.md   ← this document
```

### 11.3 Build Targets

```bash
# Build boot-rom only → distribute as sdboot.rom
cargo build -p boot-rom --release
cargo run -p tools -- mkrom target/boot-rom.bin sdboot.rom

# Build flash-rom WITH embedded boot-rom (standalone device)
BOOTROM_BIN=sdboot.rom cargo build -p flash-rom --release
espflash flash target/flash-rom.bin

# Build flash-rom WITHOUT embedded boot-rom (SD-only device)
cargo build -p flash-rom --release
espflash flash target/flash-rom.bin
```

### 11.4 CYD Development Board

The ESP32-2432S028R ("Cheap Yellow Display") is the recommended development board.

| CYD Feature | Flashpoint Mapping |
|---|---|
| ESP32-WROOM-32 | Target chip — `hal/esp32.rs` |
| ILI9341 TFT LCD | `display_flush()` / `display_clear()` |
| Resistive touch | 4 zones → BtnUp / Down / Left / Right + centre = BtnSelect |
| microSD slot | MMC partition + FAT32 partition |
| RGB LED | Optional battery low indicator |
| No PSRAM | Set `FRAME_POOL_COUNT=4` during development |

> The CYD has no PSRAM. Set `FRAME_POOL_COUNT=4` for dev. The Xteink X4 (ESP32-S3, production target) has PSRAM. Apps developed against the CYD HAL run unmodified on the Xteink once the HAL implementation is swapped.

---

## 12. Implementation Roadmap

### Phase 0 — Foundation (start here)

Everything in Phase 0 must work before any other phase begins.

| Task | Description | Priority |
|---|---|---|
| `mkrom` tool | CLI wrapping a binary with a valid Flashpoint header | P0 |
| `stage1/build.rs` | Generates `BOOTROM_OFFSET`, `BOOTROM_SIZE`, `NVS_OFFSET` as `cargo:rustc-env` | P0 |
| Stage 1 loader | SD init, FatFS mount, header validate, load into PSRAM, jump to entry point | P0 |
| Fallback chain | Magic check → SD fallback → recovery scan → panic | P0 |
| CYD HAL stub | Enough display + input to verify boot on hardware | P0 |
| Minimal boot-rom stub | Boots, renders "Flashpoint OK", halts. Package as `sdboot.rom`. | P0 |

### Phase 1 — Kernel Core

| Task | Description | Priority |
|---|---|---|
| card-ram paging layer | Page table, frame pool, LRU eviction, raw sector I/O | P1 |
| Hibernate / resume | Full state flush to MMC, resume detection on boot | P1 |
| KernelFS / NVS init | Namespace setup, `sys` + `wifi` + `app_<n>` isolation | P1 |
| `app.ini` parser | Minimal INI parser — immutable `[app]`, mutable `[tile]` | P1 |
| App scanner | Walk `/apps/`, build tile cache | P1 |

### Phase 2 — Runtimes

| Task | Description | Priority |
|---|---|---|
| wasm3 integration | Link C lib, inject host API, call `init`/`on_event`/`shutdown` | P2 |
| Lua 5.4 integration | Link C lib, inject globals, same lifecycle contract | P2 |
| Filesystem host API | `fs_open/read/write/close/exists/delete` sandboxed to `data/` | P2 |
| Display host API | `draw_bitmap`, `draw_text`, `draw_rect`, `display_flush` | P2 |
| Tile helper API | `tile_set_text/badge/color`, `tile_save` | P2 |
| Secure storage API | `secure_read/write/delete` with namespace isolation | P2 |

### Phase 3 — Shell

| Task | Description | Priority |
|---|---|---|
| Status bar | Flash %, clock, SD % | P3 |
| Battery bar | ADC read, visual bar + % display | P3 |
| App grid / launcher | Icon + tile label grid, button navigation | P3 |
| System dropdown | Hibernate, refresh, storage info, about | P3 |
| App launch / teardown | Runtime init, event loop, graceful shutdown | P3 |

### Phase 4 — Polish & Ports

| Task | Description | Priority |
|---|---|---|
| Xteink X4 HAL | eInk display driver, physical button mapping | P4 |
| WiFi Lua extension | KernelFS wifi namespace, connect / disconnect | P4 |
| Multi-platform rom | `platform=0xFF` header + slice selection in Stage 1 | Future |
| RP2040 HAL | Community port — spec compliance test suite needed first | Community |

---

## 13. Agent Handoff

> **Read this section first if picking up this project in a new session.**

### 13.1 What Flashpoint Is

- An open embedded OS platform specification for ESP32-class microcontrollers.
- A chainloadable boot system: Stage 1 (immutable, minimal) loads a `boot-rom` from SD or internal flash.
- A dual-runtime app platform: apps are WASM or Lua, declared in `app.ini`.
- A platform spec — not just one device's firmware. Community ports to other hardware are first-class.

### 13.2 What Has Been Decided — Do Not Revisit Without Good Reason

- **Language:** Rust (`esp-idf-sys` / std, Xtensa toolchain via `espup`). C libraries (wasm3, Lua 5.4) linked via `cc` crate.
- **flash-rom owns everything hardware:** Stage 1 loader, all device drivers, HAL trait implementation, and optional embedded boot-rom. Burned once.
- **boot-rom is hardware-agnostic:** Calls `Platform` trait only. Never touches hardware. The same boot-rom binary runs on any compliant device.
- **Internal flash layout:** ESP-IDF partition table (`partitions_cyd.csv`). Two named ROM slots (`flashpoint_rom` for embedded, `flashpoint_user` for SD-updated). Slot sizes tracked in NVS. Stage 1 uses `esp_partition_find()` — no hardcoded offsets.
- **Header is 64 bytes (v2).** Magic `FLPT` at `0x00`, end magic `FLPE` at `0x3C`. CRC32 at `0x1A`. PayloadType at `0x1E`. ROM ID namespace (24 bytes) at `0x1F`. Compat platforms (3 bytes) at `0x37`. API version (`built_against` u32) at `0x08`. Loaders must reject `header_size > 64`.
- **Three payload types:** `native` (XIP from flash), `wasm32` (wasm3 interpreter), `luac54` (Lua 5.4 — no NVS access).
- **ROM ID:** Null-terminated ASCII namespace (max 23 chars). Used as NVS key prefix for native/WASM payloads. Lua payloads denied NVS entirely.
- **Boot priority:** Recovery key → SD (compare+update flashpoint_user) → flashpoint_user → flashpoint_rom → Recovery Menu.
- **Feature flags:** `flash-rom` publishes `DEVICE_FEATURES` bitmask. Boot-rom header declares `required_features`. Loader enforces `(provided & required) == required` before any code runs.
- **SD boot file is `flashpoint.rom`.**
- **WASM runtime:** wasm3 — interpreter, ~64 KB, no JIT.
- **SD card:** MBR with 1 GB raw MMC partition (type `0xDA`) + FAT32 remainder (type `0x0B`).
- **Paging:** 4096-byte pages, LRU eviction, PSRAM frame pool, raw sector I/O.
- **KernelFS:** ESP-IDF NVS in `flashpoint_nvs` partition. Namespaced. Never exposed as raw file handles.
- **App format:** `/apps/<n>/{app.ini, app.wasm|app.lua, icon.bmp, data/}`
- **`app.ini`:** `[app]` immutable at runtime. `[tile]` mutable via helpers only.
- **Single app at a time.** No scheduler. No concurrent runtimes.
- **Shell owns all rendering.** Backgrounded apps have zero display access.
- **Dev board:** CYD (ESP32-2432S028R). Production target: Xteink X4 (ESP32-S3).
- **HAL strategy:** use existing crates or C libraries where fit for purpose; write custom driver code only where nothing adequate exists.
- **Build orchestration:** xtask. `cargo xtask build-flash` / `build-boot` / `flash` handles multi-target compilation. `build.rs` handles constants only.
- **DRAM is not executable on ESP32.** Native ROMs must run via XIP from flash (`flashpoint_user` or `flashpoint_rom` partition). WASM/Lua ROMs are interpreted and can be loaded from SD into heap memory directly.

### 13.3 Key Architectural Invariants

- **Stage 1 cannot brick the device.** Every validation failure falls back gracefully.
- **The kernel never gives an app a raw path into KernelFS.** Only typed helpers with namespace enforcement.
- **The `boot-rom` and `flashpoint.rom` are the same binary.** Same build output, different packaging.
- **A `flash-rom` without an embedded `boot-rom` is a valid shipping artifact.** `BOOTROM_SIZE == 0` is legal. Device waits for SD card.
- **HAL is the only hardware-aware code in `flash-rom`.** The `boot-rom` is fully portable.
- **`NVS_OFFSET` is always correct.** `build.rs` places NVS right after whatever precedes it, with or without a `boot-rom`.
- **Feature check happens before any boot-rom code runs.** A boot-rom requiring unavailable hardware is silently rejected and the next source (embedded or SD) is tried.

### 13.4 Open Questions — Status

| Question | Context | Status |
|---|---|---|
| License | Open spec intended. | **DECIDED** — Dual license. `flash-rom` (Stage 1) = copyleft (must be open, must refer to original source for forks). Custom `boot-rom` / `flashpoint.rom` builds = permissive (encouraged but not required to publish source). |
| SD boot filename | Could be `flashpoint.rom`, versioned, or as-is | **DECIDED** — `flashpoint.rom`. All references to `sdboot.rom` in this document should be read as `flashpoint.rom`. |
| Font format | What format does `draw_text` consume? Bitmap? TrueType subset? | Decide in Phase 2 before display API impl. Leaning toward compiled-in bitmap fonts for Phase 2, PSF fonts as Phase 4 upgrade. |
| eInk refresh strategy | Partial vs full refresh? Per-call vs explicit flush? | Decide when writing Xteink HAL in Phase 4. Leaning toward partial by default, `display_clear()` triggers full refresh. |
| App signing | Should `flashpoint.rom` or apps be signed? Key management? | Consider for Phase 4 / security pass. Leaning toward optional signing. |

### 13.5 Current Status (spec 0.2 — 2026-04-19)

Phase 0 is largely complete. The project is ready to implement Plan 04b (WASM runtime) and Plan 04c (partition table + 5-step boot logic). See `HANDOFF.md` in the repo root for a detailed next-session briefing.

**What is complete:**
- Plans 01–04, 06b: repo scaffold, header v2, build system, stage1 logic, QEMU E2E boot
- Hardware boot proven on CYD rev3.1 (no-ROM halt path verified)
- All 34 host tests pass (common, xtask, firmware)

**What is next (Plan 04b + 04c):**
- `partitions_cyd.csv` — 4 MB partition table with dual ROM slots
- Stage 1 redesign — 5-step boot priority, NVS slot size tracking, size constraint checks
- Recovery Menu — UART text menu for when no bootable ROM is present
- WASM runtime — wasm3 C library integration (Plan 04b)

**Test commands:**
```sh
cargo test -p common
cargo test -p xtask
cargo test -p firmware --no-default-features
cargo xtask emu-run         # QEMU full boot
cargo xtask verify flashpoint.rom
```

---

## 14. Glossary

| Term | Definition |
|---|---|
| `app.ini` | INI-format manifest for a Flashpoint app. `[app]` immutable, `[tile]` mutable. |
| `boot-rom` | The Flashpoint OS binary. Distributed as `flashpoint.rom` or embedded in a `flash-rom`. |
| `BOOTROM_OFFSET` | Compile-time constant: byte offset of embedded `boot-rom` in internal flash. 0 if none. |
| `BOOTROM_SIZE` | Compile-time constant: byte size of embedded `boot-rom`. 0 if none. |
| `build.rs` | Rust build script that emits compile-time flash layout constants. |
| `card-ram` | 1 GB raw MMC partition on SD card. Used for virtual paging and hibernate state. |
| `chainload` | Stage 1 loading and jumping to a `boot-rom` from SD card or internal flash. |
| `CYD` | Cheap Yellow Display — ESP32-2432S028R. Recommended development board. |
| `flash-rom` | Firmware burned to internal ESP32 flash. Contains Stage 1 + optional `boot-rom`. Also called `firmware`. |
| `flashpoint.rom` | A `boot-rom` packaged with a Flashpoint v2 header. SD card filename for chainloading. |
| `flashpoint_rom` | ESP-IDF partition holding the embedded (build-time) `boot-rom`. |
| `flashpoint_user` | ESP-IDF partition holding the SD-updated `boot-rom` slot. |
| `frame pool` | Fixed set of in-PSRAM page frames. The resident working set of the paging system. |
| `HAL` | Hardware Abstraction Layer. The `Platform` Rust trait all ports must implement. |
| `hibernate` | Full system state flush to `card-ram` followed by power-off. Resumed transparently on next boot. |
| `KernelFS` | Internal NVS-backed secure storage in `flashpoint_nvs` partition. Never raw file handles. |
| `MMC` | Manual Memory Control — raw sector I/O layer over the `card-ram` partition. |
| `NVS` | Non-Volatile Storage. ESP-IDF's wear-levelled key-value store used for KernelFS. |
| `NVS_OFFSET` | Compile-time constant: byte offset of NVS in internal flash. |
| `page table` | In-SRAM array of `page_entry_t` structs mapping logical page IDs to MMC sectors. |
| `PayloadType` | Enum in the header (`0x1E`): `native`, `wasm32`, `luac54`. Determines how Stage 1 boots the ROM. |
| `Platform trait` | The Rust trait defining the complete HAL contract a port must implement. |
| `Recovery Menu` | UART text menu entered when no bootable ROM is found or recovery key is held at boot. |
| `ROM ID` | 24-byte null-terminated ASCII namespace in the header (`0x1F`). Used as NVS key prefix. |
| `shell` | The Flashpoint UI: status bar, battery bar, system dropdown, app launcher grid. |
| `Stage 1` | Minimal immutable loader in internal flash. Implements 5-step boot priority. |
| `Stage 2` | The `boot-rom`. Kernel, shell, runtimes — everything above bare hardware init. |
| `tile` | Passive display descriptor for an app. Shell-rendered. Set by app via helper API only. |
| `wasm3` | Lightweight WebAssembly interpreter (~64 KB) used as the WASM runtime in Flashpoint. |
| `Xteink X4` | Production target hardware. ESP32-S3, eInk display, physical buttons, microSD. |