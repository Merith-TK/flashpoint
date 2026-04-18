# Plan 14 — Shell UI

> **Phase:** 3 — Shell
> **Prerequisites:** Plan 10 (app model + tile cache), Plan 13 (host API — display), Plan 11 or 12 (at least one runtime)
> **Estimated scope:** ~800 lines, UI rendering + navigation + app launch/teardown

---

## Objective

Implement the Flashpoint shell — the user-facing UI that renders the status bar, battery indicator, app launcher grid, system dropdown, and manages app launch/teardown. The shell owns all display rendering; backgrounded apps have zero display access.

## Shell Layout (from designdoc §10.1)

```
┌──────────────────────────────────────────────┐
│ [FLASH 23%]      12:34      [SD 67%]         │  ← status bar
├──────────────────┬───────────────────────────┤
│ ██████░░░ 72%    │ ▼ System                  │  ← battery bar + dropdown toggle
├──────────────────┴───────────────────────────┤
│                                              │
│  [icon]  [icon]  [icon]  [icon]  [icon]      │
│  label   label   label   label   label       │
│                                              │
│  [icon]  [icon]  [icon]  [icon]  [icon]      │
│  label   label   label   label   label       │
│                                              │
└──────────────────────────────────────────────┘
```

## Implementation Steps

### Status Bar

- [ ] Left region: internal flash usage % (query NVS used/total)
- [ ] Center: system time HH:MM (from RTC or millis counter since boot — no RTC on CYD, show uptime)
- [ ] Right region: SD card FAT32 usage % (query FatFS free clusters)
- [ ] Render as single-line text bar at top of display
- [ ] Update: on change only (not every frame)

### Battery Bar

- [ ] Read `Platform::battery_percent()` → visual bar + percentage text
- [ ] CYD has no battery → always shows 100% (no bar needed, or show "USB" indicator)
- [ ] Configurable low-battery threshold: emit `Event::BatteryLow` when crossed
- [ ] On production hardware (Xteink): real ADC reading, visual bar proportional to %

### App Launcher Grid

- [ ] Read tile cache (from Plan 10) → render grid of app tiles
- [ ] Each tile: icon (BMP) + label (text) + optional badge overlay
- [ ] Grid layout: calculate tile size + spacing based on display dimensions
- [ ] CYD (320×240): probably 4-5 tiles per row, 2 rows visible
- [ ] Navigation: BtnUp/Down/Left/Right moves selection cursor
- [ ] BtnSelect on a tile → launch that app
- [ ] Scrolling: if more apps than visible tiles, scroll grid vertically
- [ ] Empty state: "No Apps" centered message

### Icon Rendering (BMP)

- [ ] Parse 1-bit or greyscale BMP files from `/apps/<appname>/icon.bmp`
- [ ] Scale to fit tile icon space (letterbox, no stretch — per designdoc)
- [ ] BMP parser: minimal — support uncompressed BMP only (BI_RGB)
- [ ] Cache decoded icons in memory (small — icons are tiny)

### System Dropdown

- [ ] Toggle: BtnBack from launcher, or dedicated "System" button at top-right
- [ ] Dropdown overlays on top of launcher grid
- [ ] Menu items:
  - Display refresh (full clear — removes ghosting on eInk, noop on TFT)
  - Sleep / Hibernate → calls hibernate flow (Plan 08)
  - Storage info: flash %, SD %, card-ram used/total
  - App manager: view active app, force close
  - WiFi settings (placeholder until Phase 4)
  - About: platform version, spec version, chip ID, device ID
- [ ] Navigate with BtnUp/Down, BtnSelect to activate, BtnBack to close dropdown

### App Launch / Teardown

- [ ] On BtnSelect over a tile:
  1. Read `app.ini` → determine type (wasm/lua)
  2. Init appropriate runtime (Plan 11 or 12)
  3. Set `AppContext.foregrounded = true`
  4. Call `app.init()`
  5. Enter app event loop: `poll_event()` → `on_event(evt)` on each input
  6. BtnBack (or kernel decision) → begin teardown
- [ ] Teardown:
  1. Call `app.shutdown()`
  2. Set `AppContext.foregrounded = false`
  3. Flush tile dirty flag (write app.ini if tile.save() was called)
  4. Destroy runtime, free resources
  5. Re-render launcher grid (tile text/badges may have changed)
- [ ] Force close (from App Manager): skip `shutdown()` call, just destroy runtime

### Event Loop Architecture

```rust
enum ShellState {
    Launcher,          // grid view, navigating tiles
    Dropdown,          // system menu open
    AppRunning(AppContext, Box<dyn Runtime>),  // app has display control
}

loop {
    if let Some(evt) = platform.poll_event() {
        match &mut state {
            Launcher   => handle_launcher_event(evt),
            Dropdown   => handle_dropdown_event(evt),
            AppRunning => {
                if evt == BtnBack { teardown(); state = Launcher; }
                else { runtime.call_event(evt); }
            }
        }
    }
    platform.sleep_ms(10);  // ~100Hz poll rate
}
```

## Acceptance Criteria

- Launcher displays all discovered apps with icons and labels
- Navigation with directional buttons works fluidly
- Selecting an app launches it, BtnBack returns to launcher
- System dropdown opens and all menu items are reachable
- Hibernate from dropdown works (Plan 08 prerequisite)
- Tile updates from running app (text, badge) reflected immediately on return to launcher
- Force close from App Manager terminates a misbehaving app without crash
- Empty app list shows clean "No Apps" state

## Notes

- The shell is the most visually complex component. On CYD (TFT), rendering is fast — full screen redraws are instant. On eInk (Xteink, Phase 4), we'll need to minimize redraws. Design for "dirty region" updates now even if CYD doesn't need them.
- The launcher grid is the first thing users see after boot. It should look polished. Spend time on spacing, alignment, and text truncation with ellipsis.
- Event loop is single-threaded. `sleep_ms(10)` between polls is fine for button input. For more responsive touch (CYD), might reduce to 5ms.
- BtnBack behavior: in launcher → open dropdown. In app → return to launcher. In dropdown → close dropdown. This needs to be consistent and documented.
