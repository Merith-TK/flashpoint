# Plan 15 — Polish & Ports (Phase 4)

> **Phase:** 4 — Polish & Ports
> **Prerequisites:** Plans 01–14 (all prior phases complete)
> **Estimated scope:** Multiple parallel workstreams, ongoing

---

## Objective

Harden the platform, implement production hardware support (Xteink X4), build the bundled Lua extensions, and prepare for community ports. This is the "ship it" phase.

## Workstreams

### 15a. Xteink X4 HAL

The production target. ESP32-S3, eInk display, physical buttons, PSRAM, microSD.

- [ ] Implement `Platform` trait for Xteink X4
- [ ] eInk display driver (SPI) — partial refresh + full refresh
- [ ] Physical rocker button mapping → Event enum
- [ ] PSRAM init — `FRAME_POOL_COUNT=16` (64KB frame pool)
- [ ] Battery ADC → `battery_percent()`
- [ ] SD card via SDMMC (not SPI mode — S3 has native SDMMC)
- [ ] Verify: same `flashpoint.rom` (platform=esp32-s3) boots and renders correctly

**eInk refresh strategy (open question from designdoc §13.4):**
- **Recommendation:** `display_flush()` does a partial refresh by default. Apps call `display_clear()` for a full refresh (removes ghosting). Shell's "Display refresh" dropdown item also triggers full refresh.
- Partial refresh: ~300ms. Full refresh: ~2s. Apps should minimize full refreshes.

### 15b. WiFi Lua Extension

- [ ] Implement the `wifi` kernel extension module
- [ ] `wifi.connect()` — reads SSID + PSK from KernelFS `wifi` namespace
- [ ] `wifi.disconnect()`, `wifi.status()`, `wifi.ip()`
- [ ] `wifi.scan()` → returns list of visible networks (future)
- [ ] WiFi settings UI in system dropdown: connect, enter credentials, forget network
- [ ] Credentials stored in KernelFS `wifi` namespace — never exposed to apps
- [ ] HTTP client extension? (`http.get(url)`, `http.post(url, body)`) — depends on demand

### 15c. JSON + Crypto Extensions

- [ ] `json.encode(table) → string` — Lua table to JSON string
- [ ] `json.decode(string) → table` — JSON string to Lua table
- [ ] `crypto.sha256(data) → hex_string`
- [ ] `crypto.hmac_sha256(key, data) → hex_string`
- [ ] Consider: `crypto.random(n) → string` (n random bytes)
- [ ] Minimal implementations — no external library dependencies if possible

### 15d. App Signing (Future Consideration)

- [ ] Should `flashpoint.rom` files be signed? Who holds the keys?
- [ ] Should apps be signed? Optional or mandatory?
- [ ] Key management: device-specific keys vs. publisher keys vs. platform keys
- [ ] **Recommendation:** Optional signing in v1.0. `flashpoint.rom` header gets an optional signature field (or a separate `.sig` file). Stage 1 validates if present, ignores if absent. Apps unsigned for now.

### 15e. Multi-Platform boot-roms (Future)

- [ ] `platform=0xFF` in header → payload is a platform index + per-platform slices
- [ ] Stage 1 reads its own `chip_id()`, selects matching slice from index
- [ ] Only valid for `flashpoint.rom` files (too large for internal flash typically)
- [ ] Spec extension: define index format, slice header, alignment rules

### 15f. RP2040 Community Port

- [ ] Define `Platform` trait compliance test suite (hardware-agnostic tests)
- [ ] Document HAL contract precisely enough for community implementation
- [ ] RP2040 has no native SD MMC — will need SPI-mode SD
- [ ] RP2040 has 264KB SRAM, no PSRAM — `FRAME_POOL_COUNT` likely 4-8
- [ ] Display and input TBD by the community builder
- [ ] Flashpoint spec compliance badge for verified ports

### 15g. Developer Experience

- [ ] App template generator: `flashpoint new --type lua myapp` → scaffold `app.ini`, `app.lua`, `icon.bmp`
- [ ] Emulator / simulator for host development (SDL2 backend implementing `Platform` trait?)
- [ ] Documentation site: spec, API reference, tutorials, porting guide
- [ ] Example apps: clock, todo list, weather (with wifi), calculator, notepad

## Acceptance Criteria (Phase 4 minimum viable)

- Xteink X4 boots same `flashpoint.rom` as CYD (with correct platform ID)
- eInk display renders launcher and apps correctly
- WiFi extension connects to a network using KernelFS credentials
- JSON/crypto extensions work in Lua apps
- At least one example app demonstrates the full API surface

## Notes

- Phase 4 is intentionally open-ended. Not all workstreams need to complete before "v1.0".
- The Xteink X4 HAL (15a) and WiFi extension (15b) are the highest priority items in Phase 4.
- Community ports (15f) can begin as soon as the `Platform` trait is stable — which it should be after Phase 3.
- App signing (15d) is a security pass that should happen before any "store" or distribution mechanism is built.
