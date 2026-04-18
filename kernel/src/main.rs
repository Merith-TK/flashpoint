// kernel — Flashpoint OS kernel
//
// Hardware-agnostic. Zero hardware code here. All I/O goes through the
// Platform trait pointer published by firmware at PLATFORM_PTR_ADDR
// before jumping to this entry point.
//
// Future sub-modules (added when implemented):
//   mod kernel;   — Phase 1: card-ram paging, FatFS, NVS, event loop
//   mod shell;    — Phase 3: status bar, battery bar, app grid, dropdown
//   mod runtime;  — Phase 2: wasm3 integration, Lua 5.4 VM, host API

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use common::{Event, FrameBuffer, Platform, PLATFORM_PTR_ADDR};

// ─── Entry point ─────────────────────────────────────────────────────────────

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn entry() -> ! {
    // SAFETY: firmware wrote a valid &dyn Platform fat-pointer (2 words) to
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
    let mut row = [0u8; 640]; // max 320 x 2 bytes

    for y in 0..h {
        render_row(y, h, w, &mut row[..w as usize * 2]);
        platform.display_flush(&FrameBuffer { y, data: &row[..w as usize * 2] }).ok();
    }

    loop {
        if let Some(Event::BtnSelect) = platform.poll_event() {
            platform.reboot();
        }
        platform.sleep_ms(50);
    }
}

fn render_row(y: u16, h: u16, w: u16, row: &mut [u8]) {
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
