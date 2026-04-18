// boot-rom — Flashpoint OS kernel
//
// Hardware-agnostic. Zero hardware code here. All I/O goes through the
// Platform trait pointer published by flash-rom at PLATFORM_PTR_ADDR
// before jumping to this entry point.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod kernel;
pub mod shell;
pub mod runtime;

use flashpoint_common::{Event, Platform, PLATFORM_PTR_ADDR};

// ─── Entry point ─────────────────────────────────────────────────────────────

/// Called by Stage 1 after it validates and loads this binary.
/// Reads the Platform fat-pointer from the agreed handoff address.
#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn entry() -> ! {
    // SAFETY: Stage 1 wrote a valid &dyn Platform fat-pointer (2 words) to
    // PLATFORM_PTR_ADDR before jumping here.
    let platform: &dyn Platform = unsafe {
        &**(PLATFORM_PTR_ADDR as *const *const dyn Platform)
    };

    boot_main(platform)
}

pub fn boot_main(platform: &dyn Platform) -> ! {
    platform.display_clear().ok();

    let w = platform.display_width();
    let h = platform.display_height();
    let mut row = [0u8; 640]; // max 320 × 2 bytes

    for y in 0..h {
        render_row(y, h, w, &mut row[..w as usize * 2]);
        platform.display_flush(&flashpoint_common::FrameBuffer {
            y,
            data: &row[..w as usize * 2],
        }).ok();
    }

    loop {
        if let Some(Event::BtnSelect) = platform.poll_event() {
            platform.reboot();
        }
        platform.sleep_ms(50);
    }
}

/// Fill one scanline with the boot screen pattern.
/// Draws a centred solid bar for the "FLASHPOINT OK" text band.
fn render_row(y: u16, h: u16, w: u16, row: &mut [u8]) {
    // Background: dark navy  0x000F in RGB565
    // Text band (middle 20%): white 0xFFFF
    let text_top    = h * 2 / 5;
    let text_bottom = h * 3 / 5;
    let color: u16 = if y >= text_top && y < text_bottom { 0xFFFF } else { 0x000F };
    let bytes = color.to_le_bytes();
    for i in (0..w as usize * 2).step_by(2) {
        row[i]     = bytes[0];
        row[i + 1] = bytes[1];
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
