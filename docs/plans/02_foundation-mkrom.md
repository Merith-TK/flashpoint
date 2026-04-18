# Plan 02 — mkrom CLI Tool

> **Phase:** 0 — Foundation
> **Prerequisites:** Plan 01 (repo scaffold)
> **Estimated scope:** Single Rust binary, ~200 lines, host-only (x86)

---

## Objective

Build the `mkrom` CLI tool that wraps a raw boot-rom binary payload with a valid Flashpoint header, producing `flashpoint.rom`. This is the first deliverable because everything downstream needs valid rom files to test against.

## Flashpoint Header Format (from designdoc §4.1)

The v1 header is exactly **64 bytes**. Payload starts immediately after at byte `header_size` (= 64 for v1). The `header_size` + `header_end` fields make the format extensible for future spec versions.

| Field | Offset | Type | Size | Value |
|-------|--------|------|------|-------|
| `magic` | `0x00` | `u8[6]` | 6 | `BROM\x00\x01` |
| `spec_version` | `0x06` | `u16 LE` | 2 | `1` (current spec) |
| `platform` | `0x08` | `u8` | 1 | `0x01`=ESP32, `0x02`=ESP32-S3, `0x03`=RP2040 |
| `rom_version` | `0x09` | `u8[3]` | 3 | `[major, minor, patch]` from CLI args |
| `flags` | `0x0C` | `u16 LE` | 2 | Bit 0: compressed. Rest reserved (0). |
| `required_features` | `0x0E` | `u64 LE` | 8 | Hardware capability bitmask (see designdoc §4.4) |
| `payload_len` | `0x16` | `u32 LE` | 4 | Byte length of payload |
| `checksum` | `0x1A` | `u8[32]` | 32 | SHA-256 of payload bytes |
| `header_size` | `0x3A` | `u16 LE` | 2 | Total header bytes. `64` for v1. Payload at this offset. |
| `reserved` | `0x3C` | `u8[3]` | 3 | All zeros |
| `header_end` | `0x3F` | `u8` | 1 | Must be `0xFE` — terminator, always last byte of header |
| **Total (v1)** | | | **64 bytes** | Payload begins at byte 64 |

## CLI Interface

```bash
# Basic usage
mkrom --platform esp32 --version 0.1.0 boot-rom.bin flashpoint.rom

# Arguments
mkrom [OPTIONS] <INPUT> <OUTPUT>

Options:
  --platform <PLATFORM>        esp32 | esp32-s3 | rp2040          [required]
  --version <VERSION>          Semantic version X.Y.Z              [required]
  --requires <FEATURES>        Comma-separated feature names       [optional]
                               e.g. --requires psram,wifi,display_tft
  --compress                   Set compressed flag (future use)    [optional]

Positional:
  INPUT                        Raw boot-rom binary path
  OUTPUT                       Output .rom file path
```

## Implementation Steps

- [ ] Add `sha2` crate dependency to `tools/Cargo.toml` (for SHA-256)
- [ ] Add `clap` crate dependency (for argument parsing)
- [ ] Use feature flag constants from `flashpoint-common` — do not duplicate them in `tools`
- [ ] Define header struct with explicit byte offsets — 64 bytes total for v1
- [ ] Implement header serialization: write active fields at correct offsets, zero reserved bytes, set `header_size = 64`, set `header_end = 0xFE`
- [ ] Implement `main()`: parse args → resolve feature bitmask from `--requires` → read input → compute SHA-256 → write header + payload
- [ ] Add `--verify` subcommand: reads a `.rom` file and prints parsed header + checksum validation + human-readable feature list
- [ ] Write integration tests:
  - Round-trip: `mkrom` a dummy payload → `mkrom --verify` → all fields match
  - Reject empty input
  - Reject invalid platform string
  - Verify SHA-256 matches `sha256sum` of payload
  - Verify output file size = 64 + payload size
  - Verify `required_features` round-trips through `--requires` → `--verify`

## Acceptance Criteria

- `cargo build -p tools` succeeds on host
- `mkrom` produces a valid 64-byte header followed by the raw payload
- Active header fields at correct offsets; reserved bytes zero; `header_size == 64`; `header_end == 0xFE`
- `mkrom --verify` correctly parses and validates any `flashpoint.rom`
- SHA-256 checksum matches independent verification
- Tests pass: `cargo test -p tools`

## Key Decisions

- **Output filename default:** `flashpoint.rom` (changed from `sdboot.rom` per user decision)
- **No compression implementation yet.** The flag exists in the header but `--compress` just sets the bit. Actual compression is a future concern.
- **`--verify` subcommand** is critical for debugging Stage 1 header validation in later plans.

## Edge Cases

- Payload larger than `u32::MAX` bytes → error (theoretical, 4GB+ boot-rom impossible)
- Platform string case-insensitive: `ESP32` = `esp32`
- Version parsing: must be exactly `X.Y.Z` with each component 0-255
