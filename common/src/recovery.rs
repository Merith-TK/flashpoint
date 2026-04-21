use crate::constants::*;
use crate::gfx::{display_fill, display_text, draw_text_row, text_x_center};
use crate::io::Platform;
use crate::types::{Event, FrameBuffer};
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
// ─── Recovery menu ───────────────────────────────────────────────────────────

/// Recovery menu items.  The list is fixed; capability-gated items are shown
/// or hidden at runtime based on `platform.features()`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RecoveryItem {
    DisplayTest,
    TouchCalib,
    LedTest,
    WifiAp,   // only shown when FEAT_WIFI
    UsbMount, // only shown when FEAT_USB_OTG
    Reboot,
}

impl RecoveryItem {
    /// RGB565 colour for the band when this item is the active selection.
    fn color_active(self) -> u16 {
        match self {
            RecoveryItem::DisplayTest => 0xF81F, // magenta
            RecoveryItem::TouchCalib => 0x07FF,  // cyan
            RecoveryItem::LedTest => 0xFFE0,     // yellow
            RecoveryItem::WifiAp => 0x001F,      // blue
            RecoveryItem::UsbMount => 0x07E0,    // green
            RecoveryItem::Reboot => 0xF800,      // red
        }
    }
    /// Dimmed (inactive) version: shift right 2 bits per channel.
    fn color_inactive(self) -> u16 {
        let c = self.color_active();
        let r = (c >> 11) & 0x1F;
        let g = (c >> 5) & 0x3F;
        let b = c & 0x1F;
        ((r >> 2) << 11) | ((g >> 2) << 5) | (b >> 2)
    }
    fn label(self) -> &'static str {
        match self {
            RecoveryItem::DisplayTest => "DISPLAY TEST",
            RecoveryItem::TouchCalib => "TOUCH CALIBRATION",
            RecoveryItem::LedTest => "LED TEST",
            RecoveryItem::WifiAp => "WIFI AP RECOVERY",
            RecoveryItem::UsbMount => "USB MOUNT SD",
            RecoveryItem::Reboot => "REBOOT",
        }
    }
}

// ─── UART recovery input ─────────────────────────────────────────────────────

/// Map a UART byte to a navigation Event.
/// Supports: w/k=Up, s/j=Down, enter/space=Select, q/ESC=Back.
#[cfg(not(feature = "no-uart-recovery"))]
fn uart_byte_to_event(byte: u8) -> Option<Event> {
    match byte {
        b'w' | b'W' | b'k' | b'K' => Some(Event::BtnUp),
        b's' | b'S' | b'j' | b'J' => Some(Event::BtnDown),
        b'a' | b'A' | b'h' | b'H' => Some(Event::BtnLeft),
        b'd' | b'D' | b'l' | b'L' => Some(Event::BtnRight),
        b'\r' | b'\n' | b' ' => Some(Event::BtnSelect),
        b'q' | b'Q' | 0x1B => Some(Event::BtnBack),
        _ => None,
    }
}

/// Check for a direct numeric selection (keys '1'-'9') from UART.
/// Returns the 0-based item index, or None.
#[cfg(not(feature = "no-uart-recovery"))]
fn uart_byte_to_index(byte: u8, item_count: usize) -> Option<usize> {
    if byte >= b'1' && byte <= b'9' {
        let idx = (byte - b'1') as usize;
        if idx < item_count {
            return Some(idx);
        }
    }
    None
}

/// Unified input: polls hardware events first, then UART.
/// Returns (Option<Event>, Option<raw_uart_byte>).
#[cfg(not(feature = "no-uart-recovery"))]
fn poll_recovery_input(platform: &dyn Platform) -> (Option<Event>, Option<u8>) {
    if let Some(e) = platform.poll_event() {
        return (Some(e), None);
    }
    if let Some(byte) = platform.uart_poll_byte() {
        return (uart_byte_to_event(byte), Some(byte));
    }
    (None, None)
}

/// Simple any-input check: returns true if hardware or UART produced any event.
/// Used by recovery actions that just need "wait for any key/touch".
fn any_recovery_input(platform: &dyn Platform) -> bool {
    if platform.poll_event().is_some() {
        return true;
    }
    #[cfg(not(feature = "no-uart-recovery"))]
    if platform.uart_poll_byte().is_some() {
        return true;
    }
    false
}

/// Log the recovery menu over UART so serial users can see the options.
#[cfg(not(feature = "no-uart-recovery"))]
fn uart_log_menu(items: &[RecoveryItem], selected: usize) {
    log::info!("[recovery] ─── RECOVERY MENU ───");
    for (i, item) in items.iter().enumerate() {
        let marker = if i == selected { ">>" } else { "  " };
        log::info!("[recovery] {} [{}] {}", marker, i + 1, item.label());
    }
    log::info!(
        "[recovery] Navigate: w/s or k/j | Select: Enter/Space | Direct: 1-{}",
        items.len()
    );
}

/// Hardware-agnostic recovery menu.
///
/// - If the platform has a display (`FEAT_DISP_TFT`), renders a colour-band
///   menu and navigates with touch/button events.
/// - Otherwise falls back to the serial console: interactive UART menu.
///
/// UART console access is always active (both display and console paths)
/// unless the `no-uart-recovery` build feature is set.
pub fn recovery_main(platform: &dyn Platform) -> ! {
    log::info!("[recovery] entering recovery mode");

    let has_display = platform.features() & FEAT_DISP_TFT != 0;
    let has_wifi = platform.features() & FEAT_WIFI != 0;
    let has_usb_otg = platform.features() & FEAT_USB_OTG != 0;

    if has_display {
        recovery_display_menu(platform, has_wifi, has_usb_otg)
    } else {
        recovery_console(platform, has_wifi, has_usb_otg)
    }
}

fn build_recovery_items(has_display: bool, has_wifi: bool, has_usb_otg: bool) -> Vec<RecoveryItem> {
    let mut items: Vec<RecoveryItem> = Vec::new();
    if has_display {
        items.push(RecoveryItem::DisplayTest);
        items.push(RecoveryItem::TouchCalib);
    }
    items.push(RecoveryItem::LedTest);
    if has_wifi {
        items.push(RecoveryItem::WifiAp);
    }
    if has_usb_otg {
        items.push(RecoveryItem::UsbMount);
    }
    items.push(RecoveryItem::Reboot);
    items
}

fn recovery_display_menu(platform: &dyn Platform, has_wifi: bool, has_usb_otg: bool) -> ! {
    let items = build_recovery_items(true, has_wifi, has_usb_otg);

    let mut selected: usize = 0;
    let w = platform.display_width();
    let h = platform.display_height();
    let n = items.len() as u16;
    let band = h / n;

    // Draw initial menu + log over UART
    recovery_draw_menu(platform, &items, selected, w, h, band);
    #[cfg(not(feature = "no-uart-recovery"))]
    uart_log_menu(&items, selected);

    loop {
        // Unified input: hardware events + UART (unless no-uart-recovery)
        #[cfg(not(feature = "no-uart-recovery"))]
        {
            let (event, raw_byte) = poll_recovery_input(platform);
            // Direct number key selection
            if let Some(byte) = raw_byte {
                if let Some(idx) = uart_byte_to_index(byte, items.len()) {
                    selected = idx;
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                    platform.sleep_ms(50);
                    continue;
                }
            }
            match event {
                Some(Event::BtnUp) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnDown) => {
                    if selected + 1 < items.len() {
                        selected += 1;
                    }
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnSelect) => {
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                }
                _ => {}
            }
        }
        #[cfg(feature = "no-uart-recovery")]
        match platform.poll_event() {
            Some(Event::BtnUp) => {
                if selected > 0 {
                    selected -= 1;
                }
                recovery_draw_menu(platform, &items, selected, w, h, band);
            }
            Some(Event::BtnDown) => {
                if selected + 1 < items.len() {
                    selected += 1;
                }
                recovery_draw_menu(platform, &items, selected, w, h, band);
            }
            Some(Event::BtnSelect) => {
                recovery_run_item(platform, items[selected]);
                recovery_draw_menu(platform, &items, selected, w, h, band);
            }
            _ => {}
        }
        platform.sleep_ms(50);
    }
}

fn recovery_draw_menu(
    platform: &dyn Platform,
    items: &[RecoveryItem],
    selected: usize,
    w: u16,
    h: u16,
    band: u16,
) {
    let mut row = [0u8; 640];
    let n = items.len() as u16;
    for y in 0..h {
        let item_idx = ((y / band) as usize).min(n as usize - 1);
        let active = item_idx == selected;
        let bg = if active {
            items[item_idx].color_active()
        } else {
            items[item_idx].color_inactive()
        };
        // Fill row with band colour.
        let b = bg.to_le_bytes();
        for i in (0..w as usize * 2).step_by(2) {
            row[i] = b[0];
            row[i + 1] = b[1];
        }
        // Overlay text label centred vertically within the band.
        let band_start = item_idx as u16 * band;
        let text_top = band_start + band.saturating_sub(8) / 2;
        if y >= text_top && y < text_top + 8 {
            let label = items[item_idx].label();
            let char_row = (y - text_top) as u8;
            let lx = text_x_center(w, label);
            // Black text on bright (selected) band, white on dimmed (unselected).
            let fg: u16 = if active { 0x0000 } else { 0xFFFF };
            draw_text_row(&mut row[..w as usize * 2], lx, label, char_row, fg, bg);
        }
        platform
            .display_flush(&FrameBuffer {
                y,
                data: &row[..w as usize * 2],
            })
            .ok();
    }
}

/// Render a touch calibration target screen: black background with a cyan crosshair
/// at pixel (tx, ty) and the instruction label centred near the top.
fn recovery_cal_render(platform: &dyn Platform, tx: u16, ty: u16, label: &str) {
    let w = platform.display_width();
    let h = platform.display_height();
    let mut row = [0u8; 640];
    let b_bg = (0x0000u16).to_le_bytes();
    let b_ch = (0x07FFu16).to_le_bytes(); // cyan crosshair
    for y in 0..h {
        for i in (0..w as usize * 2).step_by(2) {
            row[i] = b_bg[0];
            row[i + 1] = b_bg[1];
        }
        if y == ty {
            for i in (0..w as usize * 2).step_by(2) {
                row[i] = b_ch[0];
                row[i + 1] = b_ch[1];
            }
        }
        let px = tx as usize * 2;
        if px + 1 < w as usize * 2 {
            row[px] = b_ch[0];
            row[px + 1] = b_ch[1];
        }
        if y >= 4 && y < 12 {
            let lx = text_x_center(w, label);
            draw_text_row(
                &mut row[..w as usize * 2],
                lx,
                label,
                (y - 4) as u8,
                0xFFFF,
                0x0000,
            );
        }
        platform
            .display_flush(&FrameBuffer {
                y,
                data: &row[..w as usize * 2],
            })
            .ok();
    }
}

/// Collect a stable touch sample: waits for 10 consecutive `poll_touch_xy()` readings
/// (50 ms apart) and returns their average. Resets the counter if the finger lifts.
/// Wait until the screen reports no touch for at least two consecutive polls.
/// Call this before `recovery_cal_sample` to flush any residual touch from
/// the previous menu tap or calibration step.
fn wait_for_no_touch(platform: &dyn Platform) {
    let mut clear = 0u32;
    while clear < 2 {
        if platform.poll_touch_xy().is_none() {
            clear += 1;
        } else {
            clear = 0;
        }
        platform.sleep_ms(50);
    }
}

fn recovery_cal_sample(platform: &dyn Platform) -> (u16, u16) {
    // Ensure any previous touch (e.g. menu selection tap) is fully lifted
    // before we start accumulating calibration samples.
    wait_for_no_touch(platform);

    const NEEDED: u32 = 10;
    let mut sum_x = 0u32;
    let mut sum_y = 0u32;
    let mut count = 0u32;
    loop {
        match platform.poll_touch_xy() {
            Some((x, y)) => {
                sum_x += x as u32;
                sum_y += y as u32;
                count += 1;
                log::info!("[cal] sample {}/{}: ({}, {})", count, NEEDED, x, y);
                if count >= NEEDED {
                    return ((sum_x / NEEDED) as u16, (sum_y / NEEDED) as u16);
                }
            }
            None => {
                if count > 0 {
                    log::warn!("[cal] lifted early ({} samples), retrying", count);
                    sum_x = 0;
                    sum_y = 0;
                    count = 0;
                }
            }
        }
        platform.sleep_ms(50);
    }
}

fn recovery_run_item(platform: &dyn Platform, item: RecoveryItem) {
    match item {
        RecoveryItem::DisplayTest => {
            log::info!("[recovery] running display test");
            let w = platform.display_width();
            let h = platform.display_height();
            let mut row = [0u8; 640];
            // Draw RGB stripes: red / green / blue / white / black
            let stripe_h = h / 5;
            let colors: [u16; 5] = [0xF800, 0x07E0, 0x001F, 0xFFFF, 0x0000];
            for y in 0..h {
                let c = colors[((y / stripe_h) as usize).min(4)];
                let b = c.to_le_bytes();
                for i in (0..w as usize * 2).step_by(2) {
                    row[i] = b[0];
                    row[i + 1] = b[1];
                }
                platform
                    .display_flush(&FrameBuffer {
                        y,
                        data: &row[..w as usize * 2],
                    })
                    .ok();
            }
            log::info!("[recovery] display test — any input to exit");
            loop {
                if any_recovery_input(platform) {
                    break;
                }
                platform.sleep_ms(50);
            }
        }
        RecoveryItem::TouchCalib => {
            // Two-point touch calibration wizard (TFT devices only).
            // Guides the user to tap a crosshair at the top-left then bottom-right
            // corners of the screen.  Raw XPT2046 ADC values at each tap are
            // averaged over 10 samples, then stored to NVS so the HAL can apply
            // accurate proportional zone mapping on the next boot.
            //
            // NVS layout — ns: "fp-hal", key: "touch-cal", 8 bytes:
            //   [x_min_lo, x_min_hi, x_max_lo, x_max_hi,
            //    y_min_lo, y_min_hi, y_max_lo, y_max_hi]
            log::info!("[recovery] entering touch calibration wizard");
            let w = platform.display_width();
            let h = platform.display_height();

            // ── Step 1: tap top-left ──────────────────────────────────────────
            log::info!("[cal] step 1/2 — tap the TOP-LEFT crosshair and hold");
            recovery_cal_render(platform, 20, 20, "TAP TOP LEFT");
            let (x1, y1) = recovery_cal_sample(platform);
            log::info!("[cal] top-left averaged raw: ({}, {})", x1, y1);
            display_fill(platform, 0x07E0); // green flash = confirmed
            platform.sleep_ms(300);

            // ── Step 2: tap bottom-right ──────────────────────────────────────
            let br_x = w.saturating_sub(21);
            let br_y = h.saturating_sub(21);
            log::info!("[cal] step 2/2 — tap the BOTTOM-RIGHT crosshair and hold");
            recovery_cal_render(platform, br_x, br_y, "TAP BOTTOM RIGHT");
            let (x2, y2) = recovery_cal_sample(platform);
            log::info!("[cal] bottom-right averaged raw: ({}, {})", x2, y2);
            display_fill(platform, 0x07E0);
            platform.sleep_ms(300);

            // ── Compute calibration bounds ─────────────────────────────────────
            let x_min = x1.min(x2);
            let x_max = x1.max(x2);
            let y_min = y1.min(y2);
            let y_max = y1.max(y2);
            log::info!(
                "[cal] calibration bounds: x {}..{}, y {}..{}",
                x_min,
                x_max,
                y_min,
                y_max
            );

            // ── Encode and write to NVS ────────────────────────────────────────
            let mut cal_bytes = [0u8; 8];
            cal_bytes[0..2].copy_from_slice(&x_min.to_le_bytes());
            cal_bytes[2..4].copy_from_slice(&x_max.to_le_bytes());
            cal_bytes[4..6].copy_from_slice(&y_min.to_le_bytes());
            cal_bytes[6..8].copy_from_slice(&y_max.to_le_bytes());

            display_fill(platform, 0x0000);
            let status = match platform.nvs_write("fp-hal", "touch-cal", &cal_bytes) {
                Ok(()) => {
                    log::info!("[cal] calibration saved to NVS — rebooting to apply");
                    "SAVED"
                }
                Err(e) => {
                    log::error!("[cal] NVS write failed: {:?}", e);
                    "NVS FAILED"
                }
            };
            let sx = text_x_center(w, status) as u16;
            display_text(platform, sx, h / 2, status, 0xFFFF, 0x0000);
            platform.sleep_ms(1500);
            platform.reboot();
        }
        RecoveryItem::LedTest => {
            log::info!("[recovery] running LED test");
            let seq: [(u8, u8, u8); 6] = [
                (255, 0, 0),
                (0, 255, 0),
                (0, 0, 255),
                (255, 255, 0),
                (255, 255, 255),
                (0, 0, 0),
            ];
            for (r, g, b) in seq {
                if platform.led_rgb(r, g, b).is_err() {
                    log::warn!("[recovery] LED not available on this device");
                    break;
                }
                platform.sleep_ms(400);
            }
        }
        RecoveryItem::WifiAp => {
            log::info!("[recovery] WiFi AP recovery — not yet implemented");
            // Future: platform.wifi_start_ap("flashpoint-recovery", "") + HTTP file server
            platform.sleep_ms(1000);
        }
        RecoveryItem::UsbMount => {
            log::info!("[recovery] USB SD mount — not yet implemented");
            // Future: expose SD card as USB mass storage device so the user can
            // transfer ROMs to/from the SD card without removing it physically.
            // Boot-ROMs may implement their own version of this via host API.
            platform.sleep_ms(1000);
        }
        RecoveryItem::Reboot => {
            log::info!("[recovery] rebooting...");
            platform.sleep_ms(500);
            platform.reboot();
        }
    }
}

fn recovery_console(platform: &dyn Platform, has_wifi: bool, has_usb_otg: bool) -> ! {
    log::info!("[recovery] ---- RECOVERY MODE (console) ----");

    #[cfg(not(feature = "no-uart-recovery"))]
    {
        // Interactive UART console: present menu, accept commands
        let items = build_recovery_items(false, has_wifi, has_usb_otg);
        let mut selected: usize = 0;
        uart_log_menu(&items, selected);

        loop {
            let (event, raw_byte) = poll_recovery_input(platform);
            // Direct number key selection
            if let Some(byte) = raw_byte {
                if let Some(idx) = uart_byte_to_index(byte, items.len()) {
                    selected = idx;
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    uart_log_menu(&items, selected);
                    platform.sleep_ms(50);
                    continue;
                }
            }
            match event {
                Some(Event::BtnUp) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnDown) => {
                    if selected + 1 < items.len() {
                        selected += 1;
                    }
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnSelect) => {
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    uart_log_menu(&items, selected);
                }
                _ => {}
            }
            platform.sleep_ms(50);
        }
    }

    // Fallback: no UART recovery — run tests automatically and reboot
    #[cfg(feature = "no-uart-recovery")]
    {
        let _ = (has_wifi, has_usb_otg);
        log::info!("[recovery] running display test...");
        platform.display_clear().ok();
        platform.sleep_ms(500);

        log::info!("[recovery] running LED test...");
        for (r, g, b) in [
            (255u8, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 255),
            (0u8, 0, 0),
        ] {
            if platform.led_rgb(r, g, b).is_err() {
                log::warn!("[recovery] LED not available");
                break;
            }
            platform.sleep_ms(400);
        }

        log::info!("[recovery] tests complete — rebooting in 3s");
        platform.sleep_ms(3000);
        platform.reboot();
    }
}
