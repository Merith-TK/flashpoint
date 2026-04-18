# Plan 04 — Stage 1 Loader

> **Phase:** 0 — Foundation
> **Prerequisites:** Plan 01 (scaffold, including `flashpoint-common`), Plan 02 (mkrom), Plan 03 (build system)
> **Estimated scope:** `no_std` Rust, ~500 lines, hardware-dependent (CYD target)

---

## Objective

Implement the Stage 1 chainload logic. Stage 1 is part of `flash-rom` — the immutable layer burned once to the device. Stage 1's sole job: find a valid boot-rom (SD card first, then internal), check feature compatibility, and jump to it. It must be unkillable — every failure path falls back gracefully.

Stage 1 also publishes the device's `DEVICE_FEATURES` bitmask to a fixed memory location before jumping, so the boot-rom can discover device capabilities at runtime.

**Architecture note:** Stage 1 is hardware-aware (it initialises SDMMC and display for error output) because it lives in `flash-rom`. The boot-rom it loads is hardware-agnostic and only ever calls the `Platform` trait provided by the `flash-rom`.

## Boot Decision Tree

```
Power On → ROM Bootloader (ESP32 built-in)
│
└── Stage 1  (part of flash-rom)
      ├── CPU / clock init (handled by ESP32 ROM + IDF startup)
      ├── Publish DEVICE_FEATURES bitmask to fixed memory location
      ├── SDMMC init
      ├── Mount FatFS partition on SD
      │
      ├── Probe SD for flashpoint.rom
      │     ├── FOUND → validate header
      │     │     ├── magic valid?                 NO → try internal
      │     │     ├── platform match?              NO → try internal
      │     │     ├── checksum valid?              NO → try internal
      │     │     ├── (provided & required) == required?  NO → try internal
      │     │     └── OK → load payload to SRAM → jump to entry
      │     └── NOT FOUND → try internal
      │
      └── Try internal
            ├── BOOTROM_SIZE == 0? → SD-only device, show error, halt
            ├── Read BOOTROM_OFFSET → check magic
            │     ├── magic valid?                 NO → recovery scan / halt
            │     ├── (provided & required) == required?  NO → halt (can't meet requirements)
            │     └── OK → jump to boot-rom entry point
            └── (never returns)
```

## Implementation Steps

### Core Logic

- [ ] Define Flashpoint header struct (shared with `mkrom` — consider a `flashpoint-common` crate)
- [ ] Implement header validation: magic check, platform check, size sanity, SHA-256 verify
- [ ] Implement SD card init (SDMMC peripheral on ESP32)
- [ ] Implement FatFS mount (FAT32 partition — partition 2 per SD layout)
- [ ] Implement `flashpoint.rom` file read from SD
- [ ] Implement payload load into execution memory
- [ ] Implement jump-to-entry (function pointer cast to boot-rom entry)
- [ ] Implement internal flash fallback path
- [ ] Implement recovery scan (optional safety net)
- [ ] Implement error display (minimal — just enough to show "NO BOOT ROM" or "CORRUPT" on screen)

### CYD-Specific Considerations

- [ ] CYD has **no PSRAM**. Stage 1 on CYD must load the boot-rom into available SRAM, or execute directly from the internal flash offset. This limits boot-rom size on CYD.
- [ ] For SD-loaded boot-roms on CYD: load to SRAM. Constrained to available SRAM after Stage 1's own usage.
- [ ] For internal boot-roms on CYD: jump directly to flash offset (XIP — execute in place).
- [ ] On PSRAM-equipped boards (ESP32-S3 / Xteink): load to PSRAM, much more headroom.

### Header Validation (exact checks)

Header is at offset 0 of the `.rom` file. Read first 64 bytes (minimum), check `header_size` at `0x3A`, read full header if needed, then payload starts at `header_size`.

```
1. len >= 64                                              → reject otherwise (too short)
2. magic == b"BROM\x00\x01"                              → reject otherwise
3. spec_version == 1                                      → reject otherwise
4. header_size >= 64 && header[header_size-1] == 0xFE    → reject otherwise (bad terminator)
5. header_size == 64 (v1 loader rejects larger headers)  → reject with "unsupported version"
6. platform == our chip_id                               → reject otherwise
7. (DEVICE_FEATURES & required_features) == required_features → reject otherwise (feature mismatch)
8. payload_len > 0 && payload_len ≤ available_memory     → reject otherwise
9. SHA-256(payload bytes) == checksum                    → reject otherwise
```

Check 7 is the feature gate — distinguishable error from a corrupt header. Check 4/5 handle forward-compatibility cleanly.

## Acceptance Criteria

- Stage 1 chainloads `flashpoint.rom` from SD card on CYD
- Stage 1 falls back to internal boot-rom if SD missing or `flashpoint.rom` absent
- Stage 1 falls back if `flashpoint.rom` header is corrupt (any field)
- Stage 1 shows a visible error state (LED blink or display text) if all boot paths fail
- Stage 1 binary size ≤ 64KB
- Zero writes to internal flash during normal SD boot (reads only)

## Open Questions for Implementation

1. **XIP vs load-to-RAM:** For internal boot-rom on CYD, XIP (execute in place from flash) avoids RAM pressure. Need to confirm ESP32 supports XIP from arbitrary SPI flash region above Stage 1.
2. **Error display on Stage 1:** Stage 1 is `no_std` and lives in `flash-rom`. The flash-rom HAL drivers are available but may not be fully initialised at Stage 1 time. Simplest path: blink RGB LED for error codes, add minimal ILI9341 text output if LED proves insufficient.

**Resolved:**
- **Shared header crate:** `flashpoint-common` holds the header struct and feature flag constants. Used by `stage1`, `flash-rom`, `boot-rom`, and `tools`. Added to Plan 01.
- **SD boot writes to flash?** No. SD-loaded boot-roms execute from SRAM. Internal flash is never written at runtime. (Designdoc §2.2 is authoritative.)

## Risk

- Stage 1 must stay small (~64KB). Every dependency matters. Use `no_std` throughout.
- SHA-256 in `no_std` needs a lightweight implementation (e.g., `sha2` crate with `no_std` feature, or hand-rolled).
