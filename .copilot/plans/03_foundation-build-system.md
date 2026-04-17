# Plan 03 — Build System (stage1/build.rs + flash-rom/build.rs)

> **Phase:** 0 — Foundation
> **Prerequisites:** Plan 01 (repo scaffold, including `flashpoint-common`)
> **Estimated scope:** Two `build.rs` files, compile-time constant generation

---

## Objective

Implement the build scripts that calculate internal flash layout offsets and emit them as compile-time constants via `cargo:rustc-env`. This is the mechanism that makes the flash layout work without a partition table.

## Constants Generated

| Constant | Source | Meaning |
|----------|--------|---------|
| `BOOTROM_OFFSET` | `stage1/build.rs` | Byte offset in internal flash where boot-rom payload begins. `0` if none embedded. |
| `BOOTROM_SIZE` | `stage1/build.rs` | Byte size of embedded boot-rom. `0` if none embedded. |
| `NVS_OFFSET` | `stage1/build.rs` | Byte offset of NVS/KernelFS region. Always correct regardless of boot-rom presence. |

## How It Works

```
WITH BOOTROM_BIN env var set:
  Stage 1 end (0x10000) → boot-rom payload (aligned 4KB) → NVS

WITHOUT BOOTROM_BIN:
  Stage 1 end (0x10000) → NVS directly
```

## Implementation Steps

### stage1/build.rs

- [ ] Read `BOOTROM_BIN` env var (optional)
- [ ] If set: read file size, align to 4KB boundary, calculate offsets
- [ ] If not set: `BOOTROM_OFFSET=0`, `BOOTROM_SIZE=0`, `NVS_OFFSET=stage1_end`
- [ ] Emit all three as `cargo:rustc-env=CONSTANT=value`
- [ ] Emit `cargo:rerun-if-env-changed=BOOTROM_BIN`
- [ ] If `BOOTROM_BIN` is set, emit `cargo:rerun-if-changed={path}` for the actual file

### flash-rom/build.rs

- [ ] This crate orchestrates the full flash image build
- [ ] If feature `embed-bootrom` is enabled: set `BOOTROM_BIN` env var pointing to the boot-rom binary, then invoke stage1 build
- [ ] Otherwise: build stage1 without `BOOTROM_BIN`
- [ ] Output: a single binary combining Stage 1 + optional boot-rom payload + NVS region marker

### Shared Constants

```rust
// stage1/src/constants.rs — consumed at compile time
pub const STAGE1_END: u32 = 0x10000;  // 64 KB
pub const NVS_SIZE: u32 = 0x40000;    // 256 KB
pub const FLASH_ALIGNMENT: u32 = 0x1000; // 4 KB
```

## Verification Steps

- [ ] Build `stage1` without `BOOTROM_BIN` → verify `BOOTROM_OFFSET=0`, `BOOTROM_SIZE=0`, `NVS_OFFSET=65536`
- [ ] Create a dummy 100KB file, set `BOOTROM_BIN` → verify `BOOTROM_OFFSET=65536`, `BOOTROM_SIZE=102400` (aligned), `NVS_OFFSET=65536+aligned_size`
- [ ] Verify alignment is always 4KB-boundary correct
- [ ] Verify `build.rs` re-runs when `BOOTROM_BIN` changes or is removed

## Acceptance Criteria

- `stage1/build.rs` emits correct constants for both with/without boot-rom scenarios
- Constants are accessible in Stage 1 source via `env!("BOOTROM_OFFSET")` etc.
- 4KB alignment is enforced on all offsets
- NVS offset is always `stage1_end + aligned_bootrom_size` (0 if no boot-rom)

## Notes

- The `align_up` function from designdoc §4.2 is the reference implementation. Use it exactly.
- Stage 1 binary size is assumed 64KB for now. If it grows, `STAGE1_END` must be bumped. This is a compile-time constant, not auto-detected.
- The boot-rom header is 64 KB (payload at `0x10000`). This is separate from the internal flash layout — `BOOTROM_OFFSET` is where the flash-rom stores the embedded boot-rom inside internal flash, not the byte offset inside the `.rom` file.

## Decision Needed: flash-rom/build.rs Coordination

### Background — what build.rs is (for Go developers)

In Go you don't have this problem because all your code targets one architecture and `go build ./...` handles everything. Here we have two completely different compilation targets that must be combined into one flash image:

- `stage1` — compiled for **Xtensa ESP32** (the microcontroller). Produces a `.bin` file.
- `flash-rom` — also compiled for **Xtensa ESP32**, but needs to *contain* the stage1 binary baked in.
- `boot-rom` (optional embed) — also Xtensa, may be baked into `flash-rom` too.

`build.rs` is a Rust script that runs on your **host machine** (x86) before the crate it belongs to is compiled. Think of it like `go generate` — it can run arbitrary code, shell commands, read files, and emit instructions to the compiler. The most common use here is setting environment variables that become compile-time constants (`env!("CONSTANT")`).

The problem: `flash-rom/build.rs` needs to trigger compilation of `stage1` for a *different* target (Xtensa) as part of building `flash-rom`. Cargo doesn't natively know how to do this across targets in one `cargo build` invocation.

---

### Option A — build.rs shells out to cargo

`flash-rom/build.rs` spawns a child `cargo build -p stage1 --target xtensa-esp32-none-elf` process, waits for it, then reads the output binary.

**Go analogy:** Like having a `go generate` directive in one package that runs `go build -o stage1.bin ./cmd/stage1`. It works but feels weird — one build tool calling itself.

```
cargo build -p flash-rom
  └── flash-rom/build.rs runs
        └── spawns: cargo build -p stage1 --target xtensa-...
              └── produces: target/xtensa-.../stage1.bin
        └── reads stage1.bin, calculates offsets, emits constants
  └── flash-rom Rust code compiles with those constants
```

- **Pros:** Single command (`cargo build -p flash-rom`) does everything. Automatic.
- **Cons:** Nested cargo calls are fragile. The inner cargo call won't automatically know to use `--release` if the outer one did. Environment variable forwarding is manual. Build caching between inner/outer cargo can conflict. Error messages get messy.

---

### Option B — pre-built binaries

`stage1` is compiled separately (by hand or a Makefile) and its `.bin` is placed in a known path. `flash-rom/build.rs` just reads it and computes offsets.

**Go analogy:** Like committing a pre-compiled `.a` library into your repo and linking against it with `cgo`. Simple, but if someone edits the stage1 source and forgets to recompile it, they get silently stale output.

```
# Step 1 (manual, or Makefile):
cargo build -p stage1 --target xtensa-esp32-none-elf --release

# Step 2 (automatic):
STAGE1_BIN=target/.../stage1.bin cargo build -p flash-rom
```

- **Pros:** `flash-rom/build.rs` stays simple. No nested cargo.
- **Cons:** Two manual steps. Easy to forget step 1 after editing stage1. Bad CI story unless you write a wrapper script anyway.

---

### Option C — xtask (recommended)

A top-level `xtask/` crate is a normal Rust binary that runs on the host and orchestrates the entire build. You run it with `cargo xtask <command>`.

**Go analogy:** Like having a `cmd/build/main.go` in your repo that you invoke with `go run ./cmd/build flash`. It's just a regular program — it can call `cargo`, run `mkrom`, copy files to the SD card path, whatever. Clean, explicit, and easy to read.

```
flashpoint/
└── xtask/
      └── src/main.rs   ← a normal host binary, runs on your laptop
```

```bash
# Build the full flash image (stage1 + embedded boot-rom):
cargo xtask build-flash --target esp32-cyd --embed-bootrom

# Build just the boot-rom as flashpoint.rom (for SD card):
cargo xtask build-rom --platform esp32 --version 0.1.0

# Flash to device:
cargo xtask flash --port /dev/ttyUSB0
```

Inside `xtask/src/main.rs`:
```rust
// This is just Rust code running on your laptop.
// It calls cargo, reads files, runs mkrom — whatever it needs.
fn build_flash(embed_bootrom: bool) {
    // 1. compile stage1 for Xtensa
    Command::new("cargo").args(["build", "-p", "stage1",
        "--target", "xtensa-esp32-none-elf", "--release"]).run();
    // 2. optionally compile boot-rom
    if embed_bootrom { ... }
    // 3. compute offsets and assemble final flash image
    // 4. write output
}
```

- **Pros:** Explicit, readable, full control. The most common pattern in serious embedded Rust projects (Embassy, Knurling/probe-rs all use xtask). Easy to add new commands later (flash, test, etc.).
- **Cons:** `cargo build -p flash-rom` alone won't produce a flashable image — you have to know to use `cargo xtask`. Minor learning curve upfront.

---

### Decision: xtask (Option C) — Locked

`flash-rom/build.rs` handles only constant generation (BOOTROM_OFFSET etc.), assuming the binaries it needs already exist. The xtask orchestrates everything: compiling stage1 for Xtensa, compiling boot-rom, running mkrom, and assembling the final flash image. See Plan 01 for the `xtask/` crate location in the repo.
