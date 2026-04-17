# Plan 08 — Hibernate / Resume

> **Phase:** 1 — Kernel Core
> **Prerequisites:** Plan 07 (card-ram paging layer)
> **Estimated scope:** ~300 lines, extends paging layer with state serialization

---

## Objective

Implement full system state flush to the card-ram MMC partition (hibernate) and transparent resume on next boot. When the user powers on after a hibernate, the system should appear exactly as it was — display restored, app state intact.

## Hibernate Flow (from designdoc §6.4)

```
Trigger: low battery / user request / graceful app close
│
├── 1. Call app shutdown() → app serializes state to /apps/<n>/data/
├── 2. Flush all dirty page frames to backing sectors
├── 3. Write page table + hibernate magic to MMC sector 0
├── 4. Write display framebuffer to reserved MMC region
├── 5. Write shell tile cache to MMC
├── 6. Flush FatFS
└── 7. Power off / deep sleep
```

## Resume Flow (Stage 2 init — from designdoc §3.2)

```
Stage 2 entry → card-ram paging init → Read MMC sector 0
│
├── Hibernate signature present?
│     YES → RESUME PATH
│     │   ├── Restore display framebuffer from MMC → immediate visual
│     │   ├── Restore shell tile cache from MMC
│     │   ├── Restore page table from sector 0
│     │   ├── Re-init runtime for last active app
│     │   ├── Call app init() → app restores own state from data/
│     │   └── Resume event loop
│     │
│     NO → FRESH BOOT PATH
│         ├── Scan /apps/ → parse each app.ini
│         ├── Build shell tile cache
│         └── Render launcher → enter event loop
```

## MMC Sector 0 Layout

```
Bytes 0–5:    Hibernate magic: "HBRT\x00\x01" (6 bytes)
Bytes 6–9:    Page table entry count (u32 LE)
Bytes 10–13:  Framebuffer MMC start sector (u32 LE)
Bytes 14–17:  Framebuffer byte length (u32 LE)
Bytes 18–21:  Tile cache MMC start sector (u32 LE)
Bytes 22–25:  Tile cache byte length (u32 LE)
Bytes 26–29:  Active app index (u32 LE, 0xFFFFFFFF if none)
Bytes 30–33:  Checksum of this header (CRC32)
Bytes 34–511: Page table entries (serialized)
              If page table overflows sector 0, continue into sector 1+
```

## Implementation Steps

### Hibernate

- [ ] Define hibernate header struct
- [ ] `hibernate()` function: orchestrates the full flush sequence
- [ ] Serialize page table entries to MMC sector 0+
- [ ] Reserve MMC sectors for framebuffer snapshot (after page table sectors)
- [ ] Write framebuffer to reserved region
- [ ] Serialize shell tile cache (app names, tile text, badge values)
- [ ] Record active app index
- [ ] CRC32 the header for corruption detection
- [ ] Call `Platform::reboot()` or deep sleep after flush

### Resume Detection

- [ ] On Stage 2 init: read MMC sector 0, check for `HBRT\x00\x01` magic
- [ ] Validate CRC32
- [ ] If valid: enter resume path
- [ ] If invalid/absent: enter fresh boot path
- [ ] **Clear hibernate magic after successful resume** — prevents boot loop if resume crashes

### Resume Path

- [ ] Deserialize page table from MMC
- [ ] Read framebuffer from MMC → `display_flush()` immediately (fast visual restore)
- [ ] Deserialize tile cache
- [ ] Set active app from header
- [ ] Return control to kernel event loop

## Acceptance Criteria

- Hibernate writes complete state to MMC
- Resume restores display within 1 second of boot
- Page table survives hibernate/resume cycle — paged data accessible after resume
- Corrupt hibernate header → clean fresh boot (no crash)
- Hibernate magic cleared after resume → next boot without hibernate is fresh

## Testing Strategy

- Mock test: hibernate → read back sector 0 → verify all fields
- Integration test on CYD: hibernate → power cycle → verify display restores
- Corruption test: write garbage to sector 0 → boot → verify fresh boot path
- Crash safety: interrupt hibernate mid-write (power cut) → next boot should fresh-boot (CRC32 catches incomplete write)

## Notes

- Framebuffer size for CYD (320×240 RGB565) = 150KB = ~300 sectors. This is a fixed reservation in the MMC partition.
- The tile cache is small (a few KB at most — app names + tile text + badge values for all installed apps).
- Hibernate is triggered by kernel, not by apps. Apps get a `shutdown()` call and must save their own state to `data/`.
- The "clear magic after resume" step is critical for crash safety. If resume itself crashes, the next boot will fresh-boot instead of looping.
