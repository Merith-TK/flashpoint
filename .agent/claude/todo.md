# Flashpoint — Agent Progress

## Phase 0: Foundation

### Step 0 — Generic / Host-Verifiable ✅ COMPLETE
- [x] Workspace `Cargo.toml` with all 7 members
- [x] `.gitignore`
- [x] `LICENSE-BOOT` (MIT) and `LICENSE-FLASH` (AGPL-3.0)
- [x] `spec/flashpoint-spec-v0.1.md`
- [x] `flashpoint-common/src/lib.rs` — header types, constants, validate_header, build_header, feature flags, ChipId, Event (9 tests pass)
- [x] `tools/src/main.rs` — mkrom pack + verify CLI (7 tests pass)
- [x] `xtask/src/main.rs` — build-rom, build-flash, flash subcommands
- [x] `stage1/build.rs` — BOOTROM_OFFSET/SIZE/NVS_OFFSET constants (3 tests pass)
- [x] `stage1/src/main.rs` — chainload logic, SD-first fallback to internal, error LED codes (hw stubs = todo!())
- [x] `flash-rom/src/hal/platform.rs` — Platform trait + FrameBuffer
- [x] `flash-rom/src/hal/esp32_cyd.rs` — CydPlatform struct, all methods stubbed todo!()
- [x] `flash-rom/src/capabilities.rs` — DEVICE_FEATURES = FEAT_DISPLAY_TFT | FEAT_INPUT_TOUCH
- [x] `boot-rom/src/main.rs` — stub entry reading PLATFORM_PTR_ADDR
- [x] `designdoc.md` updated — 64-byte header, feature flags §4.4, flash-rom/boot-rom boundary
- [x] ESP32 toolchain installed — espup, espflash v4.4.0, ldproxy v0.3.4, Xtensa Rust 1.93.0.0
- [x] `scripts/export-esp.sh` — portable env setup (uses $HOME)

### Step 0.5 — CYD Hardware Drivers ⬜ NOT STARTED
> Requires physical CYD board access. Do at PC, not SSH-from-phone.

- [ ] Verify CYD pin assignments from schematic (LCD, touch, SD, RGB LED)
- [ ] `.cargo/config.toml` — xtensa-esp32-espidf target, linker = ldproxy
- [ ] Add esp-idf-hal, esp-idf-sys, mipidsi, embedded-sdmmc deps to flash-rom/Cargo.toml
- [ ] Implement `CydPlatform` display methods (ILI9341 via mipidsi)
- [ ] Implement `CydPlatform` touch methods (XPT2046)
- [ ] Implement `CydPlatform` SD methods (embedded-sdmmc, SPI mode)
- [ ] Implement `CydPlatform` NVS methods (esp-idf-sys nvs)
- [ ] Implement `CydPlatform` trivial methods (chip_id, reboot, sleep_ms, battery_percent)
- [ ] Wire `stage1/src/main.rs` hw stubs (sd_init, flash_read, jump_to, publish_platform_ptr, error_led)
- [ ] Add stage1 Cargo.toml deps (esp-idf-sys, embedded-sdmmc, sha2 force-soft)

### Step 1 — On-Hardware Boot Test ⬜ BLOCKED on 0.5
- [ ] Build + flash stage1 + flash-rom to CYD via `cargo xtask flash`
- [ ] Test all 6 E2E scenarios from Plan 06
- [ ] Confirm boot time ≤ 3s, stage1 binary ≤ 64KB

---

## Recovery Mode Refactor — UART Console Access

### Architecture
- Core recovery logic lives in `common/src/lib.rs` (hardware-agnostic)
- HAL crates provide `uart_poll_byte()` for serial input on their UART
- HAL crates provide activation method (CYD: BOOT button, QEMU: no hardware trigger)
- All recovery paths (display + console) accept UART commands unless `no-uart-recovery` feature

### Changes
- [x] Add `uart_poll_byte()` to Platform trait in common
- [x] Add `no-uart-recovery` feature to common/Cargo.toml
- [x] Add unified `poll_recovery_input()` that checks both hardware events + UART
- [x] Add UART-to-Event mapping (w/s=up/down, enter=select, number keys=direct)
- [x] Refactor `recovery_display_menu` to use unified input + log menu state
- [x] Refactor `recovery_console` to be interactive UART menu
- [x] Add USB_MOUNT recovery menu item (FEAT_USB_OTG gated, stub)
- [x] Implement `uart_poll_byte` in hal-cyd (idf::uart_read_bytes)
- [x] Implement `uart_poll_byte` in hal-qemu (idf::uart_read_bytes)
- [x] Update docs/flashpoint/recovery-mode.md
- [x] Run `cargo test -p common` to verify

---

## Phase 1+: Kernel, Runtime, Shell — NOT STARTED
Plans exist in `.copilot/plans/07` through `15`.
