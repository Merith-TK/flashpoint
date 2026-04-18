# Plan 06b — QEMU End-to-End Boot Verification

> **Phase:** 0 — Foundation
> **Prerequisites:** Plan 02 (mkrom/pack), Plan 03 (build system), Plan 04 (Stage 1 logic)
> **Does NOT require:** Plan 05 (CYD HAL) — that's the point
> **Estimated scope:** ~200 lines across emulator + common

---

## Objective

Prove the entire Flashpoint boot chain is logically correct using only QEMU — no physical
hardware required. This lets Phase 1+ kernel development begin immediately, without
waiting for the CYD HAL (Plan 05).

The insight: Stage 1's header validation, feature-flag checking, and platform handoff are
**pure logic with no hardware dependency**. The only thing that differs between QEMU and
CYD is the `Platform` trait implementation. If the chain works in QEMU, it works on any
board that correctly implements `Platform`.

```
cargo xtask emu-run
        │
        ├─ build-boot ──► kernel binary ──► flashpoint.rom
        │                                        │
        ├─ emulator build (FLASHPOINT_ROM=...)   │
        │    └─ build.rs embeds ROM via include_bytes!
        │
        └─ QEMU launch
               │
               ▼
        [emulator/src/main.rs]
               │  EspLogger::initialize_default()
               │  let platform = EmulatorPlatform::new()
               │  validate_header(EMBEDDED_ROM, ...)   ← common::validate_header
               │  write platform ptr to PLATFORM_PTR_ADDR
               ▼
        common::boot_main(&platform)
               │  platform.display_clear()  → log::info!(...)
               │  for each scanline:
               │    platform.display_flush() → log::info!(...)
               └─ loop { platform.poll_event() → None; sleep }
```

---

## Why Not "Real" Stage 1 in QEMU

A true Stage 1 jump in QEMU would require:
- SD card image with FAT32 partition containing `flashpoint.rom`
- `esp_flash_read()` calls for internal flash fallback
- Unsafe function pointer jump to kernel entry

These are all verifiable on hardware (Plan 06) and add no additional confidence about
the boot chain *logic*. The jump itself is 2 lines; it's not where bugs hide.

This plan proves everything except the raw jump instruction — which is hardware-only and
trivially correct once the addresses are right.

---

## Changes Required

### 1. Move `boot_main()` into `common`

`boot_main()` only calls `Platform` trait methods — it has no hardware dependency.
Moving it to `common` lets both `kernel` and `emulator` call the same function,
proving the real kernel entry code runs in QEMU.

**`common/src/lib.rs`** — add:

```rust
/// Hardware-agnostic kernel entry. Called by:
///   - kernel/src/main.rs entry() on real hardware (via platform ptr handoff)
///   - emulator/src/main.rs directly (via EmulatorPlatform)
pub fn boot_main(platform: &dyn Platform) -> ! {
    platform.display_clear().ok();

    let w = platform.display_width();
    let h = platform.display_height();
    let mut row = [0u8; 640];

    for y in 0..h {
        render_row(y, h, w, &mut row[..w as usize * 2]);
        platform.display_flush(&FrameBuffer {
            y, data: &row[..w as usize * 2],
        }).ok();
    }

    loop {
        if let Some(Event::BtnSelect) = platform.poll_event() {
            platform.reboot();
        }
        platform.sleep_ms(50);
    }
}

fn render_row(y: u16, h: u16, w: u16, row: &mut [u8]) {
    let text_top    = h * 2 / 5;
    let text_bottom = h * 3 / 5;
    let color: u16 = if y >= text_top && y < text_bottom { 0xFFFF } else { 0x000F };
    let bytes = color.to_le_bytes();
    for i in (0..w as usize * 2).step_by(2) {
        row[i]     = bytes[0];
        row[i + 1] = bytes[1];
    }
}
```

**`kernel/src/main.rs`** — update `boot_main` call to `common::boot_main`:

```rust
pub extern "C" fn entry() -> ! {
    let platform: &dyn Platform = unsafe {
        &**(PLATFORM_PTR_ADDR as *const *const dyn Platform)
    };
    common::boot_main(platform)
}
```

**Why common, not kernel?** If `boot_main` lives in `kernel`, emulator would need to
depend on `kernel` as a library — which requires kernel to have both a `[lib]` and a
`[[bin]]` target, and it still carries no_std + embedded baggage into a std host build.
Putting it in `common` is clean: `common` already supports both std and no_std.

### 2. `EmulatorPlatform` in `emulator/src/platform.rs`

Implements all `Platform` trait methods using ESP-IDF APIs:

```rust
use common::{ChipId, Event, FrameBuffer, Platform, PlatformError};

pub struct EmulatorPlatform;

impl Platform for EmulatorPlatform {
    // Display: log scanline stats to UART instead of driving hardware
    fn display_clear(&self) -> Result<(), PlatformError> {
        log::info!("[display] clear");
        Ok(())
    }
    fn display_flush(&self, buf: &FrameBuffer) -> Result<(), PlatformError> {
        // Log every 60 lines so QEMU output is readable, not 240 lines of pixels
        if buf.y % 60 == 0 {
            log::info!("[display] scanline y={}", buf.y);
        }
        Ok(())
    }
    fn display_width(&self)  -> u16 { 320 }
    fn display_height(&self) -> u16 { 240 }

    // Input: no events in QEMU — boot_main loops until BtnSelect, which never comes.
    // emu-run should kill QEMU after seeing "FLASHPOINT OK" in the log.
    fn poll_event(&self) -> Option<Event> { None }

    // Trivial
    fn battery_percent(&self) -> u8  { 100 }
    fn chip_id(&self)         -> ChipId { ChipId::Esp32 }
    fn sleep_ms(&self, ms: u32) {
        use esp_idf_svc::hal::delay::FreeRtos;
        FreeRtos::delay_ms(ms);
    }
    fn reboot(&self) -> ! {
        unsafe { esp_idf_sys::esp_restart() };
        loop {}
    }

    // SD/NVS: not present in emulator
    fn sd_read_sectors(&self, _: u32, _: &mut [u8]) -> Result<(), PlatformError> {
        Err(PlatformError::SdReadError)
    }
    fn sd_write_sectors(&self, _: u32, _: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::SdWriteError)
    }
    fn sd_sector_count(&self) -> u32 { 0 }
    fn nvs_read(&self, _: &str, _: &str) -> Result<Vec<u8>, PlatformError> {
        Err(PlatformError::NvsError)
    }
    fn nvs_write(&self, _: &str, _: &str, _: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::NvsError)
    }
    fn nvs_delete(&self, _: &str, _: &str) -> Result<(), PlatformError> {
        Err(PlatformError::NvsError)
    }
}
```

### 3. Embed ROM via `build.rs` + `include_bytes!`

**`emulator/build.rs`** — after the existing `embuild` call, locate and copy the ROM:

```rust
fn main() {
    embuild::espidf::sysenv::output();

    // If FLASHPOINT_ROM is set (by xtask emu-build), copy it into OUT_DIR
    // so include_bytes! can reference it at a stable path.
    println!("cargo:rerun-if-env-changed=FLASHPOINT_ROM");
    if let Ok(rom_path) = std::env::var("FLASHPOINT_ROM") {
        println!("cargo:rerun-if-changed={rom_path}");
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap())
            .join("flashpoint.rom");
        std::fs::copy(&rom_path, &out).expect("failed to copy flashpoint.rom");
        println!("cargo:rustc-env=FLASHPOINT_ROM_PATH={}", out.display());
    }
}
```

**`emulator/src/main.rs`** — full rewrite:

```rust
use esp_idf_svc::log::EspLogger;
use common::{validate_header, PLATFORM_ESP32, PLATFORM_PTR_ADDR, HEADER_V1_SIZE};

mod platform;
use platform::EmulatorPlatform;

// Embedded at compile time by build.rs when FLASHPOINT_ROM is set.
// Falls back to a zero-length slice so the crate builds without a ROM
// (though the validation will fail — expected).
#[cfg(env = "FLASHPOINT_ROM_PATH")]
static EMBEDDED_ROM: &[u8] = include_bytes!(env!("FLASHPOINT_ROM_PATH"));
#[cfg(not(env = "FLASHPOINT_ROM_PATH"))]
static EMBEDDED_ROM: &[u8] = &[];

fn main() {
    EspLogger::initialize_default();

    log::info!("================================");
    log::info!("  FLASHPOINT  v0.1.0  [QEMU]");
    log::info!("================================");

    // Validate the embedded ROM header exactly as Stage 1 would
    match validate_header(EMBEDDED_ROM, 0 /* no required features */, PLATFORM_ESP32) {
        Ok(payload_offset) => {
            log::info!("[stage1] header OK — payload at offset {}", payload_offset);
        }
        Err(e) => {
            log::error!("[stage1] header validation failed: {:?}", e);
            log::error!("[stage1] build with FLASHPOINT_ROM set to run full E2E");
            loop {}
        }
    }

    // Publish platform pointer (same mechanism as real Stage 1)
    let platform = EmulatorPlatform;
    let platform_ref: &dyn common::Platform = &platform;
    let fat_ptr = &platform_ref as *const &dyn common::Platform as *const ();
    unsafe {
        core::ptr::write(PLATFORM_PTR_ADDR as *mut *const (), fat_ptr as *const ());
    }

    log::info!("[stage1] platform ptr published → 0x{:08X}", PLATFORM_PTR_ADDR);
    log::info!("[stage1] jumping to kernel...");
    log::info!("================================");

    // Call boot_main directly (same code as kernel entry, no unsafe jump needed in QEMU)
    common::boot_main(&platform)
}
```

### 4. Update `cargo xtask emu-build` / `emu-run`

```rust
fn cmd_emu_build(output: &Path) -> Result<(), String> {
    // Step 1: build kernel → flashpoint.rom
    let rom = workspace_root().join("flashpoint.rom");
    cmd_build_boot("esp32", "0.1.0", None, &rom)?;

    // Step 2: build emulator with ROM embedded
    println!("==> compiling emulator (FLASHPOINT_ROM={})", rom.display());
    run(esp_cmd("cargo")
        .args(["build", "-p", "emulator", "--release"])
        .env("FLASHPOINT_ROM", rom.to_str().unwrap()))?;

    // Step 3: create merged flash image
    let bin = workspace_root()
        .join("target/xtensa-esp32-espidf/release/emulator");
    println!("==> creating merged flash image → {}", output.display());
    run(Command::new("espflash")
        .args(["save-image", "--chip", "esp32", "--merge",
            bin.to_str().unwrap(),
            output.to_str().unwrap(),
        ]))
}
```

---

## Acceptance Criteria

- [ ] `cargo xtask emu-run` completes with zero manual steps
- [ ] QEMU serial output contains `[stage1] header OK`
- [ ] QEMU serial output contains `[stage1] jumping to kernel...`
- [ ] QEMU serial output contains `[display] clear` and `[display] scanline` entries
- [ ] `validate_header()` rejects a deliberately corrupted ROM (add a xtask test flag)
- [ ] `validate_header()` rejects a ROM built with `--platform esp32-s3`
- [ ] `validate_header()` rejects a ROM with `--requires psram` (device has none)

---

## What This Unlocks

Once 06b passes:
- The **full boot chain logic is proven correct** — any future bug is in a HAL driver,
  not in Stage 1 or the kernel boot sequence
- Phase 1 kernel work (Plans 07–10) can begin without hardware
- Plan 05 (CYD HAL) + Plan 06 (hardware E2E) become pure driver work with no unknowns
- `cargo xtask emu-run` becomes the standard "does it boot?" sanity check for all
  future kernel changes

## QEMU HAL — What Can Be Emulated

The `EmulatorPlatform` is a stub, but QEMU does expose real peripherals we can drive.
These are optional extensions to 06b — implement them if useful for development:

| Platform method | QEMU equivalent | Notes |
|-----------------|-----------------|-------|
| `display_flush()` | Write raw RGB565 bytes to a named pipe / file | A host-side viewer can render them live |
| `display_clear()` | Same pipe, send a "clear" sentinel byte | Simple protocol |
| `poll_event()` | Read a byte from UART RX (`$serialMonitor:TX`) | Map byte values to Events |
| `sleep_ms()` | `FreeRtos::delay_ms()` | Already works in QEMU |
| `sd_read_sectors()` | Read from a raw sector image via UART or flash region | Complex — skip for 06b |
| `nvs_read/write()` | In-memory hashmap (no NVS peripheral in QEMU) | Simple stub, useful for kernel dev |

**Recommended minimal QEMU HAL for kernel development:**
- `display_flush()` → write scanlines to a pipe → host renders to framebuffer (or just log)
- `poll_event()` → UART byte input → map to BtnUp/Down/Select/Back
- `nvs_*` → in-memory map (lets kernel store/load settings without real flash)

This gives enough to develop and test the shell UI (Plan 14) entirely in QEMU before
touching hardware.

---

## Relationship to Plan 06

Plan 06 (hardware E2E) is still required — it verifies the HAL drivers, the SPI bus
timings, the SD card read path, the XIP jump, and the physical display. Plan 06b does
not replace it. The intended sequence is:

```
06b (QEMU) → prove logic → start Phase 1+
                          ↓ (parallel, when CYD board available)
                    05 (CYD HAL) → 06 (hardware) → confirm on device
```
