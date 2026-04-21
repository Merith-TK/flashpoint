use crate::gfx::{display_fill, display_text, text_x_center};
use crate::io::Platform;
use crate::types::Event;

#[cfg(feature = "test-image")]
const TEST_IMAGE: &[u8] = include_bytes!(env!("FLASHPOINT_TEST_IMAGE"));
#[cfg(feature = "test-image")]
const TEST_IMAGE_W: u16 = 128;
#[cfg(feature = "test-image")]
const TEST_IMAGE_H: u16 = 128;

#[allow(unreachable_code)]
pub fn boot_main(platform: &dyn Platform) -> ! {
    log::info!("[boot_main] starting");
    let w = platform.display_width();
    let h = platform.display_height();
    log::info!("[boot_main] display {}x{}", w, h);

    #[cfg(feature = "test-image")]
    {
        log::info!(
            "[boot_main] rendering orientation test image ({}x{})",
            TEST_IMAGE_W,
            TEST_IMAGE_H
        );
        display_fill(platform, 0x0000);
        let x_off = (w.saturating_sub(TEST_IMAGE_W)) / 2;
        let y_off = (h.saturating_sub(TEST_IMAGE_H)) / 2;
        let row_bytes = TEST_IMAGE_W as usize * 2;
        let mut row_buf = [0u8; 640];
        for img_y in 0..TEST_IMAGE_H {
            let screen_y = y_off + img_y;
            if screen_y >= h {
                break;
            }
            for i in (0..w as usize * 2).step_by(2) {
                row_buf[i] = 0;
                row_buf[i + 1] = 0;
            }
            let src_start = img_y as usize * row_bytes;
            let src_end = src_start + row_bytes;
            let dst_start = x_off as usize * 2;
            let dst_end = dst_start + row_bytes;
            if src_end <= TEST_IMAGE.len() && dst_end <= row_buf.len() {
                row_buf[dst_start..dst_end].copy_from_slice(&TEST_IMAGE[src_start..src_end]);
            }
            platform
                .display_flush(&crate::types::FrameBuffer {
                    y: screen_y,
                    data: &row_buf[..w as usize * 2],
                })
                .ok();
        }
        display_text(
            platform,
            x_off,
            y_off.saturating_sub(10),
            "TOP (USB?)",
            0xFFFF,
            0x0000,
        );
        let bottom_y = y_off + TEST_IMAGE_H + 2;
        if bottom_y + 8 <= h {
            display_text(platform, x_off, bottom_y, "BOTTOM", 0xFFFF, 0x0000);
        }
        log::info!("[boot_main] test image rendered — looping forever");
        loop {
            platform.sleep_ms(100);
        }
    }

    display_fill(platform, 0x000F);

    let title = "FLASHPOINT";
    let tx = text_x_center(w, title) as u16;
    display_text(platform, tx, h / 3, title, 0xFFFF, 0x000F);

    let sub = "NO ROM FOUND";
    let sx = text_x_center(w, sub) as u16;
    display_text(platform, sx, h / 3 + 16, sub, 0xFD20, 0x000F);

    let hint = "HOLD BOOT TO RECOVER";
    let hx = text_x_center(w, hint) as u16;
    display_text(platform, hx, h * 3 / 4, hint, 0x07E0, 0x000F);

    log::info!("[boot_main] render complete — entering event loop");
    loop {
        if let Some(Event::BtnSelect) = platform.poll_event() {
            platform.reboot();
        }
        platform.sleep_ms(50);
    }
}
