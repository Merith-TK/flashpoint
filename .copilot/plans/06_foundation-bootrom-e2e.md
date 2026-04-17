# Plan 06 — Minimal Boot-ROM Stub + End-to-End Verification

> **Phase:** 0 — Foundation
> **Prerequisites:** Plan 02 (mkrom), Plan 03 (build system), Plan 04 (Stage 1), Plan 05 (CYD HAL)
> **Estimated scope:** Minimal boot-rom that proves the chain works end-to-end

---

## Objective

Write the smallest possible boot-rom that exercises the full boot path: Stage 1 (in `flash-rom`) chainloads it, the boot-rom calls back into the `Platform` trait for display output, renders "FLASHPOINT OK", and halts. This is the Phase 0 "it works" milestone. Once this passes, the device is unkillable and all further development ships as `flashpoint.rom`.

The boot-rom stub must contain **zero hardware-specific code** — it only calls `Platform` trait methods. This proves the HAL boundary works correctly end-to-end.

## What the Stub Does

```
Entry point (jumped to by Stage 1)
├── Init PSRAM (if available — skip on CYD)
├── Init CYD HAL (display, input, SD, NVS)
├── Clear display
├── Render centered text: "⚡ FLASHPOINT"
├── Render below: "v0.1.0 — ESP32"
├── Render below: "System OK"
├── Halt (infinite loop, polling for BtnSelect to reboot)
```

## Implementation Steps

### Build the Stub

- [ ] Create minimal `boot-rom/src/main.rs` — entry point, HAL init, display test
- [ ] Create `FrameBuffer` type (even if minimal — line buffer or full buffer depending on SRAM)
- [ ] Use CYD HAL `display_flush()` to render test output
- [ ] Simple text rendering — hardcoded bitmap font, doesn't need to be pretty
- [ ] Compile boot-rom for ESP32 target
- [ ] Package with `mkrom`: `mkrom --platform esp32 --version 0.1.0 boot-rom.bin flashpoint.rom`

### Test Scenarios

- [ ] **Happy path (SD boot):** Place `flashpoint.rom` on SD FAT32 partition → power on CYD → see "FLASHPOINT OK" on display
- [ ] **Happy path (internal boot):** Build `flash-rom` with embedded boot-rom → flash to CYD → remove SD → power on → see "FLASHPOINT OK"
- [ ] **Fallback (no SD, no internal):** Build `flash-rom` without boot-rom → remove SD → power on → Stage 1 shows error (LED blink or minimal display)
- [ ] **Fallback (corrupt ROM):** Place a garbage file named `flashpoint.rom` on SD → power on → Stage 1 rejects it → falls back to internal or errors
- [ ] **Fallback (wrong platform):** Use `mkrom --platform esp32-s3` to create mismatched ROM → Stage 1 rejects → fallback
- [ ] **SD priority:** Both SD `flashpoint.rom` and internal boot-rom present → SD wins

### End-to-End Verification Checklist

- [ ] Stage 1 binary ≤ 64KB
- [ ] `flashpoint.rom` has valid header (verify with `mkrom --verify`)
- [ ] CYD display shows expected output from boot-rom stub
- [ ] All fallback paths tested and documented
- [ ] Boot time measured: power-on to "FLASHPOINT OK" in under 3 seconds (target)

## Acceptance Criteria

- **The device boots from SD and displays the stub message.** This is the single most important milestone.
- All 6 test scenarios pass on physical CYD hardware.
- Removing or corrupting `flashpoint.rom` never bricks the device.
- The full chain is proven: `mkrom` → `flashpoint.rom` → SD card → Stage 1 → boot-rom → display.

## What This Unlocks

Once Plan 06 passes:
- Phase 0 is **COMPLETE**
- All further OS development happens in `boot-rom/` and ships as `flashpoint.rom`
- No USB cable needed for iteration (copy file to SD, reboot)
- Confidence that the device is unkillable — we can develop fearlessly

## Notes

- The stub's text rendering can be crude. A basic 8×8 bitmap font is fine. Pretty fonts come in Phase 2.
- If CYD SRAM can't hold a full 320×240 RGB565 framebuffer (150KB), use a line-buffer approach and flush line by line. This works fine for static text.
- The boot-rom stub should log to UART as well for debugging (ESP-IDF `log` macros). Not visible to user but helpful during development.
