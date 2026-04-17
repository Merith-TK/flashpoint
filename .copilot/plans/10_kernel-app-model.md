# Plan 10 — App Model (app.ini Parser + App Scanner)

> **Phase:** 1 — Kernel Core
> **Prerequisites:** Plan 06 (Phase 0 complete), Plan 09 (KernelFS — for secure namespace creation)
> **Estimated scope:** ~400 lines, INI parsing + directory walking + tile cache

---

## Objective

Implement the app discovery and manifest parsing system. The kernel scans `/apps/` on the SD FAT32 partition, parses each `app.ini`, builds an in-memory tile cache for the shell launcher, and validates app structure.

## App Directory Structure (from designdoc §8.1)

```
/apps/
  <appname>/
    app.ini       ← manifest
    app.wasm      ← if type=wasm
    app.lua       ← if type=lua
    icon.bmp      ← launcher icon
    data/         ← app private read-write sandbox
```

## app.ini Format (from designdoc §8.2)

```ini
[app]
name   = Weather
type   = wasm        ; wasm | lua
entry  = app.wasm
secure = false       ; true = request secure namespace

[tile]
text   = Loading...  ; subtitle shown under icon
color  = white       ; white | black | invert
font   = small       ; small | medium | large
icon   = icon.bmp
badge  =             ; integer or empty
```

## Implementation Steps

### INI Parser

- [ ] Minimal INI parser — no external crate dependency (keep boot-rom small)
- [ ] Parse sections: `[app]` and `[tile]`
- [ ] Parse key-value pairs: `key = value` (trim whitespace, handle comments with `;`)
- [ ] `[app]` fields: `name` (string), `type` (enum: wasm|lua), `entry` (string), `secure` (bool)
- [ ] `[tile]` fields: `text` (string), `color` (enum: white|black|invert), `font` (enum: small|medium|large), `icon` (string), `badge` (Option<u8>)
- [ ] Validation: reject `app.ini` missing required `[app]` fields. `[tile]` fields all have defaults.

### App Manifest Struct

```rust
struct AppManifest {
    // From [app] — immutable at runtime
    name: String,
    app_type: AppType,   // Wasm | Lua
    entry: String,       // "app.wasm" or "app.lua"
    secure: bool,
    dir_name: String,    // directory name under /apps/

    // From [tile] — mutable via tile helpers
    tile: TileState,
}

struct TileState {
    text: String,
    color: TileColor,    // White | Black | Invert
    font: FontSize,      // Small | Medium | Large
    icon_path: String,
    badge: Option<u8>,   // None = hidden, Some(0..99) = shown
    dirty: bool,         // needs write-back to app.ini
}
```

### App Scanner

- [ ] Walk `/apps/` directory on FAT32
- [ ] For each subdirectory: attempt to read and parse `app.ini`
- [ ] Validate entry file exists (`app.wasm` or `app.lua` as declared)
- [ ] Create `data/` directory if absent
- [ ] Build `Vec<AppManifest>` — the tile cache
- [ ] Sort apps by name for consistent launcher order
- [ ] Log warnings for malformed apps (skip them, don't crash)

### Tile Cache

- [ ] In-memory array of `TileState` for all discovered apps
- [ ] Shell reads this cache to render the launcher grid
- [ ] Tile helpers (`tile_set_text`, `tile_set_badge`, etc.) modify the cache in place
- [ ] `tile_save()` writes modified `[tile]` section back to `app.ini` on SD

### tile_save() Implementation

- [ ] Read existing `app.ini` from SD
- [ ] Replace `[tile]` section values with current `TileState`
- [ ] Write back to SD
- [ ] Clear `dirty` flag
- [ ] Only `[tile]` section is modified — `[app]` section preserved exactly

## Acceptance Criteria

- Scanner discovers all valid apps in `/apps/`
- Malformed apps are skipped with a warning (no crash)
- `app.ini` round-trip: parse → modify tile → save → parse again → values match
- `[app]` section is never modified by tile_save()
- Missing `data/` directory created automatically
- Empty `/apps/` directory → empty tile cache (valid state, shell shows "No Apps")

## Testing Strategy

- Unit test INI parser with various inputs (valid, missing sections, extra whitespace, comments)
- Unit test scanner with mock filesystem (mock FAT32 reads)
- Integration test on CYD: create test apps on SD → scan → verify tile cache contents
- Edge cases: app with no icon.bmp, app with empty badge, app with `secure=true`

## Notes

- The INI parser should be minimal. No need for full INI spec — just `[section]` + `key = value` + `;` comments. No multi-line values, no escaping, no quoted strings.
- Icon loading (BMP parsing) is not part of this plan — shell rendering (Plan 14) handles that.
- `secure=true` apps create their `app_<name>` NVS namespace via KernelFS (Plan 09) on first launch, not at scan time.
- Tile `dirty` flag is separate from any paging dirty flag. It just tracks whether `tile_save()` needs to write to SD.
