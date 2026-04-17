# Plan 12 — Lua Runtime (Lua 5.4 Integration)

> **Phase:** 2 — Runtimes
> **Prerequisites:** Plan 10 (app model), Plan 07 (card-ram)
> **Estimated scope:** ~400 lines, C FFI + globals injection + lifecycle + bundled extensions

---

## Objective

Integrate Lua 5.4 as the second application runtime. Lua apps use injected global functions and tables instead of WASM imports, but bind to the exact same kernel host API. Lua is also the language for kernel-bundled extensions (wifi, json, crypto).

## Architecture

```
App script (app.lua on SD)
     │ loaded from SD FAT32
     ▼
Lua 5.4 VM (C library, linked statically)
     │
     ├── Injected globals (same kernel functions as WASM)
     │     ├── fs.open(), fs.read(), fs.write(), ...
     │     ├── draw.bitmap(), draw.text(), draw.rect(), ...
     │     ├── tile.set_text(), tile.set_badge(), ...
     │     ├── secure.read(), secure.write(), secure.delete()
     │     └── require("wifi"), require("json"), require("crypto")
     │
     └── Global functions called by kernel
           ├── init()
           ├── on_event(evt)
           └── shutdown()
```

## Implementation Steps

### Lua C Library Integration

- [ ] Add Lua 5.4 source under `vendor/lua54/` (or download)
- [ ] Compile via `cc` crate in `boot-rom/build.rs`
- [ ] Hand-written Rust FFI bindings for essential Lua C API:
  - `luaL_newstate()`, `luaL_openlibs()`, `lua_close()`
  - `luaL_loadbuffer()`, `lua_pcall()`
  - `lua_pushcfunction()`, `lua_setglobal()`, `lua_getglobal()`
  - `lua_createtable()`, `lua_setfield()` — for namespace tables (fs, draw, tile, secure)
  - `lua_tostring()`, `lua_tointeger()`, `lua_pushlstring()`, etc.

### Runtime Lifecycle

- [ ] `LuaRuntime::new(platform, app_manifest) → Result<Self>`
  - Create Lua state, open safe standard libs (string, table, math — NOT os, io, debug)
  - Read `app.lua` from SD into memory
  - Register all host API globals
  - Load and compile the script (`luaL_loadbuffer`)
  - Execute top-level code (defines `init`, `on_event`, `shutdown` functions)
- [ ] `LuaRuntime::call_init()` — `lua_getglobal("init")` → `lua_pcall()`
- [ ] `LuaRuntime::call_event(evt)` — push event constant → `lua_pcall()`
- [ ] `LuaRuntime::call_shutdown()` — `lua_getglobal("shutdown")` → `lua_pcall()`
- [ ] `LuaRuntime::drop()` — `lua_close()`, free resources

### Host API as Lua Globals

Organized as tables to mirror the clean namespace design:

```lua
-- Filesystem (sandboxed to /apps/<appname>/data/)
fs.open(path, mode)   -- mode: "r" | "w" | "a"
fs.read(fd, len)      -- returns string
fs.write(fd, data)    -- data is string
fs.close(fd)
fs.exists(path)       -- returns boolean
fs.delete(path)

-- Display
draw.bitmap(x, y, w, h, data)
draw.text(x, y, text)
draw.rect(x, y, w, h, fill)  -- fill: "white" | "black" | "invert"
draw.flush()
draw.clear()
draw.set_font(name)  -- "small" | "medium" | "large"

-- Tile helpers
tile.set_text(s)
tile.set_badge(n)     -- -1 to clear
tile.set_color(s)     -- "white" | "black" | "invert"
tile.save()

-- Secure storage (only if secure=true in app.ini)
secure.read(key)      -- returns string or nil
secure.write(key, val)
secure.delete(key)
```

### Sandbox Security

- [ ] Do NOT open `os` library (file system access, command execution)
- [ ] Do NOT open `io` library (raw file I/O bypasses sandbox)
- [ ] Do NOT open `debug` library (can inspect/modify anything)
- [ ] Override `require()` — only allow loading bundled kernel extensions, not arbitrary Lua files
- [ ] `loadfile()`, `dofile()` → disabled
- [ ] Custom memory allocator with limit — prevent Lua GC from consuming all SRAM

### Bundled Kernel Extensions

- [ ] Implement `wifi` module (wraps KernelFS wifi namespace — never exposes credentials)
  - `wifi.connect()` — reads SSID/PSK from KernelFS, initiates connection
  - `wifi.disconnect()`
  - `wifi.status()` — returns "connected" | "disconnected" | "connecting"
  - `wifi.ip()` — returns IP string or nil
- [ ] Implement `json` module (encode/decode)
  - `json.encode(table) → string`
  - `json.decode(string) → table`
- [ ] Implement `crypto` module (basic hashing)
  - `crypto.sha256(data) → hex_string`
  - `crypto.hmac_sha256(key, data) → hex_string`
- [ ] Extensions are compiled into the boot-rom, registered via custom `require()` loader

## Acceptance Criteria

- Lua app runs on CYD: `init()` called, `draw.text()` renders, `on_event()` receives buttons
- Standard library sandboxed: `os.execute()` not available
- Custom `require()` loads only bundled extensions
- Memory limit enforced: Lua state cannot exceed configured max
- Same host API behavior as WASM (filesystem sandbox, display access, tile helpers)
- Bundled wifi extension connects using KernelFS credentials without exposing them to the script

## Notes

- Lua apps will likely be more common than WASM for hobbyist developers. Lua is simpler to write and doesn't require a cross-compiler.
- The wifi, json, and crypto extensions are Phase 4 deliverables but the extension loading mechanism should be built now.
- Lua scripts are loaded as text, not bytecode. This avoids bytecode version issues and security concerns with untrusted bytecode.
- Memory limit: 64KB is a reasonable starting point for Lua state on CYD. 256KB on PSRAM-equipped boards.
