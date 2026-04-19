# Plan 02 — mkrom CLI Tool

> **Phase:** 0 — Foundation
> **Prerequisites:** Plan 01 (repo scaffold)
> **Estimated scope:** Single Rust binary, ~200 lines, host-only (x86)

---

## Objective

Build the `mkrom` CLI tool that wraps a raw boot-rom binary payload with a valid Flashpoint header, producing `flashpoint.rom`. This is the first deliverable because everything downstream needs valid rom files to test against.

## Flashpoint Header Format

> ✅ **Implemented.** See `docs/spec/flashpoint-spec-v0.2.md §4.1` for the authoritative v2 header layout. The `xtask pack` / `xtask verify` commands in `xtask/src/rom.rs` are the canonical implementation. `common/src/lib.rs` holds all offset constants and `build_header()`.

**Key changes from original plan:** SHA-256 → CRC32, `BROM\x00\x01` magic → `FLPT`/`FLPE`, tool renamed `mkrom` → `cargo xtask pack`.

## CLI Interface (as built)

```bash
# Pack a binary into flashpoint.rom
cargo xtask pack --platform esp32 --version 0.1.0 [--type native|wasm32|luac54] [--id com.example.app] input.bin flashpoint.rom

# Verify a rom file
cargo xtask verify flashpoint.rom

# Build kernel and pack in one step
cargo xtask build-boot [--platform esp32] [--version 0.1.0] [--type native] [--id com.flashpoint.shell]
```

## Acceptance Criteria

> ✅ All met. 12 xtask tests pass. See `cargo test -p xtask`.

## Key Decisions

- **Output filename default:** `flashpoint.rom` (changed from `sdboot.rom` per user decision)
- **No compression implementation yet.** The flag exists in the header but `--compress` just sets the bit. Actual compression is a future concern.
- **`--verify` subcommand** is critical for debugging Stage 1 header validation in later plans.

## Edge Cases

- Payload larger than `u32::MAX` bytes → error (theoretical, 4GB+ boot-rom impossible)
- Platform string case-insensitive: `ESP32` = `esp32`
- Version parsing: must be exactly `X.Y.Z` with each component 0-255
