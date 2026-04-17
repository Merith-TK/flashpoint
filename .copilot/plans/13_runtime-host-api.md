# Plan 13 — Host API Surface

> **Phase:** 2 — Runtimes
> **Prerequisites:** Plan 11 (WASM runtime), Plan 12 (Lua runtime), Plan 09 (KernelFS), Plan 07 (card-ram)
> **Estimated scope:** ~600 lines, shared kernel implementations backing both WASM + Lua APIs

---

## Objective

Implement the shared kernel-side host API functions that both WASM imports and Lua globals call into. This is the single source of truth — runtime-specific bindings (wasm3 callbacks, Lua C functions) are thin wrappers around these shared implementations.

## Design Principle

```
WASM import callback ──┐
                       ├──→ Shared kernel host function ──→ HAL / KernelFS / card-ram
Lua C function ────────┘
```

One implementation. Two binding layers. Zero behavior divergence.

## API Groups

### 1. Filesystem API

All paths sandboxed to `/apps/<appname>/data/`. Path traversal (`..`) rejected.

| Function | Signature | Behavior |
|----------|-----------|----------|
| `host_fs_open` | `(app_ctx, path, mode) → fd` | Open file. mode: Read/Write/Append. Returns fd or error. |
| `host_fs_read` | `(fd, buf, len) → bytes_read` | Read up to `len` bytes. Returns 0 at EOF, -1 on error. |
| `host_fs_write` | `(fd, buf, len) → bytes_written` | Write `len` bytes. Returns written count or -1. |
| `host_fs_close` | `(fd)` | Close file descriptor. |
| `host_fs_exists` | `(app_ctx, path) → bool` | Check if file exists in sandbox. |
| `host_fs_delete` | `(app_ctx, path)` | Delete file from sandbox. |

**Security:**
- [ ] Normalize paths: collapse `.`, reject `..`, reject absolute paths
- [ ] Prepend `/apps/<appname>/data/` to all paths
- [ ] Maximum open file descriptors per app: 4 (configurable)
- [ ] File descriptor table per app context — closed on app shutdown

### 2. Display API

Only functional while the app is foregrounded. Kernel revokes access on background/shutdown.

| Function | Signature | Behavior |
|----------|-----------|----------|
| `host_draw_bitmap` | `(x, y, w, h, data_ptr)` | Blit 1-bit or greyscale bitmap to framebuffer |
| `host_draw_text` | `(x, y, text)` | Render text at pixel coords with current font |
| `host_draw_rect` | `(x, y, w, h, fill)` | Draw rectangle. fill: white/black/invert |
| `host_display_flush` | `()` | Push framebuffer to display hardware via HAL |
| `host_display_clear` | `()` | Fill framebuffer white |
| `host_set_font` | `(font_id)` | Select font: small(0) / medium(1) / large(2) |

**Implementation details:**
- [ ] Framebuffer is kernel-owned. Apps write to it, kernel flushes via HAL.
- [ ] `draw_text` needs a bitmap font renderer. Three sizes: 8px, 12px, 16px (compiled-in bitmap fonts).
- [ ] `draw_bitmap` reads pixel data from WASM linear memory (for WASM) or Lua string (for Lua).
- [ ] Coordinate bounds checking: clamp to display dimensions, don't crash on out-of-bounds.

### 3. Tile Helper API

| Function | Signature | Behavior |
|----------|-----------|----------|
| `host_tile_set_text` | `(app_ctx, text)` | Update tile text in shell cache |
| `host_tile_set_badge` | `(app_ctx, value)` | Set badge (0-99) or clear (-1) |
| `host_tile_set_color` | `(app_ctx, color)` | Set tile color: white/black/invert |
| `host_tile_save` | `(app_ctx)` | Persist tile state to app.ini on SD |

### 4. Secure Storage API

Only available if `app.ini` has `secure = true`. Otherwise, calls return error.

| Function | Signature | Behavior |
|----------|-----------|----------|
| `host_secure_read` | `(app_ctx, key) → Option<bytes>` | Read from app's KernelFS namespace |
| `host_secure_write` | `(app_ctx, key, value)` | Write to app's KernelFS namespace |
| `host_secure_delete` | `(app_ctx, key)` | Delete from app's KernelFS namespace |

## App Context

Every host API call receives an `AppContext` that identifies the calling app:

```rust
struct AppContext {
    app_name: String,
    app_dir: String,          // "/apps/<appname>"
    data_dir: String,         // "/apps/<appname>/data"
    secure_enabled: bool,     // from app.ini [app] secure
    nvs_handle: Option<AppNamespaceHandle>,  // from KernelFS
    open_fds: [Option<FileHandle>; 4],       // fd table
    foregrounded: bool,       // display access flag
}
```

## Implementation Steps

- [ ] Define `AppContext` struct and construction from `AppManifest`
- [ ] Implement filesystem API with path sandboxing and fd table
- [ ] Implement display API with framebuffer management and font rendering
- [ ] Implement tile helper API (updates in-memory cache from Plan 10)
- [ ] Implement secure storage API (delegates to KernelFS from Plan 09)
- [ ] Implement display access revocation: set `foregrounded = false` → display calls return error
- [ ] Create WASM binding layer: `m3_LinkRawFunction` wrappers calling shared functions
- [ ] Create Lua binding layer: `lua_pushcfunction` wrappers calling shared functions
- [ ] Verify both bindings produce identical behavior for identical inputs

## Acceptance Criteria

- WASM app and Lua app calling the same host API produce identical results
- Filesystem sandbox cannot be escaped via `..` or absolute paths
- Display calls fail gracefully when app is backgrounded
- Secure storage calls fail gracefully when `secure = false`
- File descriptors cleaned up on app shutdown (no leaks)
- Font rendering works for all three sizes

## Font Strategy (decision needed)

Options:
1. **Compiled-in bitmap fonts** — small/medium/large as static byte arrays. Simplest, smallest footprint. Monospace only.
2. **BDF/PCF fonts** — parsed at runtime. More flexible, slightly larger footprint.
3. **PSF fonts** — Linux console font format. Simple parser, good coverage.

**Recommendation:** Compiled-in bitmap fonts for Phase 2. PSF fonts as an upgrade in Phase 4 if needed.

## Notes

- The host API is the security boundary. Every function must validate inputs. Never trust pointers, lengths, paths, or values from the app runtime.
- WASM pointer validation: wasm3 provides access to linear memory base + size. All ptr+len pairs must fall within this range.
- Lua string handling: Lua strings are immutable and length-counted. No null-terminator issues, but watch for large allocations.
