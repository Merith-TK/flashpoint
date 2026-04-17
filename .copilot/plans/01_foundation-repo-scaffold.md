# Plan 01 — Repository Scaffold

> **Phase:** 0 — Foundation
> **Prerequisites:** None
> **Estimated scope:** Directory structure, Cargo workspace, license files, gitignore, CI skeleton

---

## Objective

Create the full repository layout from designdoc §11.2 as a Cargo workspace. Every crate compiles (even if empty). License files reflect the dual-license decision.

## Directory Structure

```
flashpoint/
├── Cargo.toml              ← workspace root (all members listed)
├── LICENSE-FLASH            ← copyleft license for flash-rom / Stage 1
├── LICENSE-BOOT             ← permissive license for boot-rom / apps
├── .gitignore
├── flashpoint-common/       ← shared types: header struct, feature flags, ChipId, Event
│     ├── Cargo.toml         ← no_std compatible, no hardware deps
│     └── src/
│           └── lib.rs       ← RomHeader, FeatureFlags consts, ChipId, Event enums
├── stage1/                  ← minimal chainload loader, part of flash-rom (no_std)
│     ├── Cargo.toml
│     ├── build.rs           ← placeholder, implemented in plan 03
│     └── src/
│           └── main.rs      ← placeholder
├── boot-rom/                ← OS kernel (Rust, std via esp-idf) — hardware-agnostic
│     ├── Cargo.toml
│     └── src/
│           ├── main.rs
│           ├── kernel/
│           │     └── mod.rs
│           ├── shell/
│           │     └── mod.rs
│           └── runtime/
│                 └── mod.rs
├── flash-rom/               ← device firmware: Stage 1 + HAL drivers + optional embedded boot-rom
│     ├── Cargo.toml
│     ├── build.rs           ← placeholder, implemented in plan 03
│     └── src/
│           ├── main.rs
│           ├── capabilities.rs   ← DEVICE_FEATURES bitmask
│           └── hal/
│                 ├── mod.rs
│                 ├── platform.rs    ← Platform trait definition
│                 └── esp32_cyd.rs   ← CYD impl (plan 05)
├── tools/
│     ├── Cargo.toml
│     └── src/
│           └── mkrom.rs     ← placeholder, implemented in plan 02
├── xtask/                   ← build orchestration (runs on host, not ESP32)
│     ├── Cargo.toml
│     └── src/
│           └── main.rs      ← placeholder; commands: build-flash, build-rom, flash
└── spec/
      └── flashpoint-spec-v0.1.md  ← symlink or copy of designdoc
```

## Implementation Steps

- [ ] Create `Cargo.toml` workspace root with all members
- [ ] Create `flashpoint-common/` crate — `no_std` compatible lib, no hardware deps; contains `RomHeader` struct, `FeatureFlags` bitmask constants, `ChipId` enum, `Event` enum
- [ ] Create `stage1/` crate — `no_std`, Xtensa target, depends on `flashpoint-common`, placeholder main
- [ ] Create `boot-rom/` crate — `std` (esp-idf), depends on `flashpoint-common`, module tree with empty `mod.rs`
- [ ] Create `flash-rom/` crate — `std` (esp-idf), depends on `flashpoint-common`; contains `Platform` trait, HAL module tree, `capabilities.rs` stub
- [ ] Create `tools/` crate — host binary, depends on `flashpoint-common`, for `mkrom`
- [ ] Create `xtask/` crate — host binary, build orchestration; placeholder `main.rs` with stubbed `build-flash`, `build-rom`, `flash` subcommands
- [ ] Create `spec/` directory — copy or symlink designdoc
- [ ] Create `LICENSE-FLASH` (AGPL-3.0 or similar copyleft — requires source disclosure + attribution)
- [ ] Create `LICENSE-BOOT` (MIT or Apache-2.0 — permissive, encourages but doesn't require source)
- [ ] Create `.gitignore` (Rust targets, IDE files, build artifacts, `*.rom`)
- [ ] Verify `cargo check -p tools -p flashpoint-common` passes on host

## Acceptance Criteria

- All crates listed in workspace `Cargo.toml`
- `cargo check -p tools -p flashpoint-common` passes on host
- `flashpoint-common` exports `RomHeader`, `FeatureFlags` consts, `ChipId`, `Event`
- Module tree for `flash-rom` and `boot-rom` matches designdoc §11.2
- Both license files present with correct text
- `.gitignore` covers `target/`, `*.rom`, IDE artifacts

## Notes

- `stage1`, `boot-rom`, and `flash-rom` target Xtensa — they won't `cargo check` on x86. Structure validity is what matters here.
- `tools/mkrom` and `flashpoint-common` are host-compatible and should check on x86.
- `Platform` trait lives in `flash-rom/src/hal/platform.rs`. The `boot-rom` calls it via a function pointer or trait object passed at entry. The exact calling convention is decided in Plan 04/06.
- `flash-rom` is the device-specific crate. `boot-rom` is the portable OS. They are separate binaries.
