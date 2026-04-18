# Plan 05 — CYD HAL Implementation

> **Phase:** 0 — Foundation
> **Prerequisites:** Plan 01 (scaffold), Plan 04 (Stage 1 — informs hardware init patterns)
> **Estimated scope:** `Platform` trait implementation for CYD, ~400 lines

---

## Objective

Implement the `Platform` trait for the ESP32-2432S028R (CYD) development board inside `flash-rom`. This HAL is the boundary between raw hardware and the boot-rom — everything above the `Platform` trait is hardware-agnostic.

**Approach:** Use an existing crate or C library wherever one exists and is fit for purpose. Where none exists or available options are inadequate, write the minimum necessary driver code. The `Platform` trait is the contract — how it is satisfied is an implementation detail.

| Layer | Preferred source |
|-------|-----------------|
| Display | `mipidsi` (ILI9341 driver) + `display-interface-spi` |
| Touch | `xpt2046` crate or equivalent; custom if nothing suitable exists |
| SD card (raw sector I/O) | `embedded-sdmmc` |
| NVS / KernelFS | `esp-idf-sys` NVS API (C bindings, already available) |
| SPI bus | `esp-idf-hal` SPI peripheral |

Pin assignments must be verified against the actual CYD schematic before writing any init code.

## CYD Hardware Map

| CYD Feature | Flashpoint HAL Method | Driver |
|---|---|---|
| ESP32-WROOM-32 | `chip_id()` → `ChipId::Esp32` | N/A |
| ILI9341 TFT LCD (320×240) | `display_flush()`, `display_clear()`, `display_width()`, `display_height()` | SPI, DC/RST GPIOs |
| Resistive touch panel (XPT2046) | `poll_event()` → button zone mapping | SPI (separate bus from LCD) |
| microSD slot | `sd_read_sectors()`, `sd_write_sectors()`, `sd_sector_count()` | SDMMC or SPI-mode SD |
| NVS (internal flash) | `nvs_read()`, `nvs_write()`, `nvs_delete()` | ESP-IDF NVS API |
| RGB LED | Optional — battery low indicator | GPIO |
| No PSRAM | `FRAME_POOL_COUNT=4` | N/A |
| No battery | `battery_percent()` → always 100 | N/A (stub) |

## Platform Trait (from designdoc §5.1)

```rust
pub trait Platform {
    // Storage
    fn sd_read_sectors(&self, start: u32, buf: &mut [u8]) -> Result<()>;
    fn sd_write_sectors(&self, start: u32, buf: &[u8])    -> Result<()>;
    fn sd_sector_count(&self) -> u32;
    fn nvs_read(&self, ns: &str, key: &str)              -> Result<Vec<u8>>;
    fn nvs_write(&self, ns: &str, key: &str, val: &[u8]) -> Result<()>;
    fn nvs_delete(&self, ns: &str, key: &str)            -> Result<()>;

    // Display
    fn display_flush(&self, buf: &FrameBuffer) -> Result<()>;
    fn display_clear(&self)                    -> Result<()>;
    fn display_width(&self)  -> u16;
    fn display_height(&self) -> u16;

    // Input
    fn poll_event(&self) -> Option<Event>;

    // System
    fn battery_percent(&self) -> u8;
    fn chip_id(&self)         -> ChipId;
    fn reboot(&self)          -> !;
    fn sleep_ms(&self, ms: u32);
}
```

## Implementation Steps

### Display (ILI9341 via SPI)

- [ ] Init SPI bus for LCD (CYD-specific pins: MOSI, CLK, CS, DC, RST)
- [ ] ILI9341 init sequence (command list for 320×240, landscape/portrait TBD)
- [ ] `display_flush()` — blit `FrameBuffer` to display via SPI DMA
- [ ] `display_clear()` — fill white (or black, configurable)
- [ ] `display_width()` → 320, `display_height()` → 240

### Input (XPT2046 Resistive Touch → Button Zones)

- [ ] Init SPI bus for touch controller (separate from LCD SPI)
- [ ] Read raw X/Y coordinates from XPT2046
- [ ] Map touch regions to directional buttons:
  ```
  ┌────────────────────────┐
  │         UP             │
  ├───────┬────────┬───────┤
  │ LEFT  │ SELECT │ RIGHT │
  ├───────┴────────┴───────┤
  │        DOWN            │
  └────────────────────────┘
  ```
- [ ] Debounce: require stable reading for N ms before emitting event
- [ ] `poll_event()` → returns `Some(Event::BtnXxx)` or `None`
- [ ] Handle BtnBack: long-press SELECT, or dedicated zone (decide during impl)

### Storage (SD Card)

- [ ] Init SDMMC peripheral (or SPI-mode SD — CYD may require SPI mode)
- [ ] Raw sector read/write for card-ram partition
- [ ] `sd_sector_count()` — query card capacity
- [ ] FatFS mount handled by kernel, not HAL — HAL just provides raw sector I/O

### NVS

- [ ] Init NVS flash at `NVS_OFFSET` via `esp-idf-sys` NVS API
- [ ] Implement `nvs_read/write/delete` wrapping ESP-IDF `nvs_get_blob`/`nvs_set_blob`/`nvs_erase_key`
- [ ] Namespace parameter maps directly to NVS namespace

### System

- [ ] `battery_percent()` → `100` (CYD has no battery, stub)
- [ ] `chip_id()` → `ChipId::Esp32`
- [ ] `reboot()` → `esp_restart()` (ESP-IDF call, never returns)
- [ ] `sleep_ms()` → `vTaskDelay()` or `std::thread::sleep()`

## Acceptance Criteria

- CYD displays a solid color or test pattern via `display_flush()`
- Touch input registers as correct directional events
- SD card raw sector read/write works (test with known data pattern)
- NVS read/write/delete round-trips correctly
- `chip_id()` returns `ChipId::Esp32`
- All trait methods implemented (no `unimplemented!()` left)

## CYD Pin Reference

```
LCD (ILI9341):  MOSI=13, CLK=14, CS=15, DC=2, RST=? (verify from CYD schematic)
Touch (XPT2046): MOSI=?, CLK=?, CS=?, IRQ=? (separate SPI bus, verify from schematic)
SD Card:        CMD=?, CLK=?, D0=? (SDMMC or SPI, verify from schematic)
RGB LED:        R=4, G=16, B=17 (verify)
Backlight:      GPIO=21 (verify)
```

> Pin assignments MUST be verified against the actual CYD schematic before implementation. The above are common values but may vary by CYD revision.

## Notes

- CYD has no PSRAM. The frame pool for card-ram will be 4 frames × 4KB = 16KB in SRAM. This is tight but workable for Phase 0.
- The FrameBuffer type needs definition. For ILI9341 at 320×240 RGB565, a full framebuffer is 150KB — too large for SRAM. We'll need a line-buffer or tile-buffer approach. Define `FrameBuffer` to support partial updates.
- BtnBack mapping is an open question. Options: long-press center, fifth touch zone (e.g., top-left corner), or a physical button if the CYD revision has one.
