# Plan 11 — WASM Runtime (wasm3 Integration)

> **Phase:** 2 — Runtimes
> **Prerequisites:** Plan 10 (app model), Plan 07 (card-ram — for loading large binaries)
> **Estimated scope:** ~500 lines, C FFI binding + host API injection + lifecycle

---

## Objective

Integrate the wasm3 WebAssembly interpreter into the boot-rom kernel. WASM apps are loaded from SD, executed in a sandboxed wasm3 runtime, and interact with the system exclusively through injected host API imports.

## Why wasm3

- Interpreter-only (~64KB footprint), no JIT — safe for microcontrollers
- No dynamic memory allocation required from host (uses pre-allocated arena)
- Suitable for eInk refresh rates (don't need speed, need small footprint)
- Mature C library, easy to link via Rust `cc` crate

## Architecture

```
App binary (app.wasm on SD)
     │ loaded via card-ram paging
     ▼
wasm3 runtime (C library, linked statically)
     │
     ├── Host API imports (env module)
     │     ├── fs_open, fs_read, fs_write, fs_close, ...
     │     ├── draw_bitmap, draw_text, draw_rect, ...
     │     ├── tile_set_text, tile_set_badge, ...
     │     ├── secure_read, secure_write, secure_delete
     │     └── (all sandboxed, all go through kernel)
     │
     └── Exported functions called by kernel
           ├── init()
           ├── on_event(evt: i32)
           └── shutdown()
```

## Implementation Steps

### wasm3 C Library Integration

- [ ] Add wasm3 as git submodule under `vendor/wasm3/` (or download source)
- [ ] Create `cc` build script in `boot-rom/build.rs` to compile wasm3 C sources
- [ ] Create Rust FFI bindings for wasm3 API (minimal, hand-written — no bindgen needed)
- [ ] Key wasm3 functions needed:
  - `m3_NewEnvironment()`, `m3_NewRuntime()`, `m3_ParseModule()`, `m3_LoadModule()`
  - `m3_LinkRawFunction()` — for injecting host API imports
  - `m3_FindFunction()` — for calling exports (init, on_event, shutdown)
  - `m3_CallV()` / `m3_Call()` — invoke exported functions

### Runtime Lifecycle

- [ ] `WasmRuntime::new(platform, app_manifest) → Result<Self>`
  - Allocate wasm3 environment + runtime with configured stack size
  - Load `app.wasm` from SD via card-ram paging (read into contiguous buffer)
  - Parse and load module
  - Link all host API functions
  - Find and cache exported function pointers (init, on_event, shutdown)
- [ ] `WasmRuntime::call_init() → Result<()>` — call app's `init()` export
- [ ] `WasmRuntime::call_event(evt: Event) → Result<()>` — call `on_event(evt as i32)`
- [ ] `WasmRuntime::call_shutdown() → Result<()>` — call `shutdown()` export
- [ ] `WasmRuntime::drop()` — free wasm3 runtime + environment, release card-ram pages

### Host API Binding (env module imports)

Each host function is a C-compatible callback registered via `m3_LinkRawFunction`. The callback receives wasm3's stack pointer and runtime context.

- [ ] **Filesystem:** `fs_open`, `fs_read`, `fs_write`, `fs_close`, `fs_exists`, `fs_delete`
  - Read/write from WASM linear memory via stack-provided pointers
  - All paths sandboxed to `/apps/<appname>/data/`
- [ ] **Display:** `draw_bitmap`, `draw_text`, `draw_rect`, `display_flush`, `display_clear`, `set_font`
  - Only functional while app is foregrounded
- [ ] **Tile:** `tile_set_text`, `tile_set_badge`, `tile_set_color`, `tile_save`
- [ ] **Secure storage:** `secure_read`, `secure_write`, `secure_delete` (only if `secure=true` in app.ini)

### Memory Safety

- [ ] WASM linear memory is fully sandboxed by wasm3 — apps cannot access host memory
- [ ] Host API callbacks validate all pointer+length pairs against WASM linear memory bounds
- [ ] Buffer overflows in WASM are caught by wasm3, not by host code
- [ ] Stack overflow in WASM → wasm3 trap → kernel catches and terminates app gracefully

## Acceptance Criteria

- Compile a trivial WASM app (Rust → wasm32-unknown-unknown) that calls `draw_text` and `display_flush`
- Load and run it on CYD via the full chain: `flashpoint.rom` → kernel → wasm3 → display output
- `init()`, `on_event()`, `shutdown()` lifecycle works correctly
- Filesystem sandbox enforced: WASM app cannot read outside `data/`
- Invalid WASM binary → graceful error, not a crash
- wasm3 OOM or stack overflow → graceful teardown

## Configuration

| Setting | Value | Notes |
|---------|-------|-------|
| wasm3 stack size | 8KB–16KB | Configurable, start with 16KB |
| wasm3 memory pages | 4–16 | Each page = 64KB of WASM linear memory |
| Max WASM binary size | 256KB | Larger binaries paged via card-ram |

## Notes

- Only ONE runtime active at a time. No concurrent WASM + Lua. Kernel enforces this.
- wasm3 is single-threaded. no async/await in WASM apps. The event-driven model (init → on_event loop → shutdown) is the only execution model.
- Actual host API implementation details are in Plan 13. This plan focuses on the wasm3 integration layer and the binding mechanism.
