# Plan 09 — KernelFS / NVS

> **Phase:** 1 — Kernel Core
> **Prerequisites:** Plan 06 (Phase 0 complete — HAL with NVS working)
> **Estimated scope:** ~250 lines, wraps HAL NVS methods with namespace policy enforcement

---

## Objective

Implement KernelFS — the internal-flash-only secure storage layer built on top of ESP-IDF NVS. KernelFS enforces strict namespace isolation: apps can only access their own namespace, and security-sensitive namespaces like `wifi` are never exposed to apps.

## Namespace Layout (from designdoc §7.1)

| Namespace | Contents | Writable By |
|-----------|----------|-------------|
| `sys` | Device ID, platform version, boot counter | Kernel only |
| `wifi` | SSID, PSK, last IP | Kernel wifi extension only |
| `ext` | Bundled Lua extension metadata | Kernel only |
| `app_<name>` | Per-app secure storage (credentials, tokens) | That app only, via secure API |

## Implementation Steps

### Core KernelFS Manager

- [ ] Define `KernelFS` struct: holds reference to `Platform`, tracks open namespaces
- [ ] `init()` — initialize NVS at `NVS_OFFSET`, create `sys` namespace if first boot, bump boot counter
- [ ] `sys_read(key) → Result<Vec<u8>>` — kernel-only read from `sys` namespace
- [ ] `sys_write(key, value) → Result<()>` — kernel-only write to `sys` namespace

### Namespace Access Control

- [ ] `open_app_namespace(app_name) → AppNamespaceHandle` — creates `app_<name>` namespace on first access, returns a scoped handle
- [ ] `AppNamespaceHandle` can only read/write/delete within its own namespace
- [ ] Attempting to access another app's namespace → error (enforced at API level, not just convention)
- [ ] `wifi` and `sys` and `ext` namespaces are NEVER accessible via `AppNamespaceHandle`

### Secure Storage API (for apps)

- [ ] `secure_read(handle, key) → Result<Option<Vec<u8>>>` — read from app's namespace
- [ ] `secure_write(handle, key, value) → Result<()>` — write to app's namespace
- [ ] `secure_delete(handle, key) → Result<()>` — delete from app's namespace
- [ ] These map directly to WASM imports and Lua globals (Plan 13)

### First Boot Setup

- [ ] Detect first boot: check for `sys/device_id` key
- [ ] If absent: generate device ID (random or chip MAC-based), write to `sys`
- [ ] Write platform version, spec version to `sys`
- [ ] Initialize boot counter at 0

### WiFi Namespace (kernel extension only)

- [ ] `wifi_read(key) → Result<Option<Vec<u8>>>` — callable only from bundled wifi Lua extension
- [ ] `wifi_write(key, value) → Result<()>` — same restriction
- [ ] Credentials stored here are NEVER exposed to apps, not even as opaque handles

## Acceptance Criteria

- `sys` namespace initialized on first boot with device ID + version + boot counter
- Boot counter increments on every boot
- App namespace isolation enforced: app "weather" cannot read from "app_notes"
- `wifi` namespace inaccessible from any app context
- Round-trip: write a key via secure API → read it back → values match
- Delete a key → subsequent read returns None

## Testing Strategy

- Unit test with mock NVS backend
- Test namespace isolation: create two app handles, verify cross-access fails
- Test first boot detection: fresh NVS → `sys` namespace populated
- Test repeated boot: boot counter increments

## Notes

- NVS has limited capacity (~256KB). Individual keys are limited to 4000 bytes. Apps should store large data in `/apps/<name>/data/` on SD, not in KernelFS.
- The `ext` namespace stores metadata about bundled Lua extensions (versions, enabled state). The extensions themselves are compiled into the boot-rom, not stored in NVS.
- Namespace names in ESP-IDF NVS are limited to 15 characters. `app_<name>` must be truncated/hashed if the app name is too long. Define a naming strategy (e.g., `app_` + first 11 chars, or `app_` + hash).
