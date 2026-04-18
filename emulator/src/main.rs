use esp_idf_svc::log::EspLogger;
use common::{
    validate_header, boot_main, Platform,
    PLATFORM_ESP32, PLATFORM_PTR_ADDR,
    FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING,
    FEAT_DISP_TFT, FEAT_INPUT_TOUCH,
};

mod platform;
use platform::EmulatorPlatform;

// Embedded at compile time by build.rs when FLASHPOINT_ROM env var is set.
// Falls back to a zero-length slice (validation will fail with TooShort — expected
// when running without a ROM, e.g. during a bare firmware-only emu-run).
static EMBEDDED_ROM: &[u8] = include_bytes!(env!("FLASHPOINT_ROM_PATH"));

// Features this emulator declares (mirrors CYD for test parity)
const DEVICE_FEATURES: u64 = FEAT_DISP_TFT | FEAT_INPUT_TOUCH;

fn main() {
    EspLogger::initialize_default();

    log::info!("================================");
    log::info!("  FLASHPOINT  v0.1.0  [QEMU]");
    log::info!("================================");

    // ── Stage 1: validate embedded ROM header ────────────────────────────────
    match validate_header(
        EMBEDDED_ROM,
        DEVICE_FEATURES,
        PLATFORM_ESP32,
        FLASHPOINT_CURRENT,
        FLASHPOINT_LAST_BREAKING,
    ) {
        Ok(payload_offset) => {
            log::info!("[stage1] header OK — payload at offset {}", payload_offset);
        }
        Err(e) => {
            log::error!("[stage1] header validation failed: {:?}", e);
            log::error!("[stage1] build with FLASHPOINT_ROM set for full E2E");
            loop {}
        }
    }

    // ── Publish platform pointer (same mechanism as real Stage 1) ────────────
    let platform = EmulatorPlatform;
    let platform_ref: &dyn Platform = &platform;
    let fat_ptr = &platform_ref as *const &dyn Platform as *const ();
    unsafe {
        core::ptr::write(PLATFORM_PTR_ADDR as *mut *const (), fat_ptr as *const ());
    }

    log::info!("[stage1] platform ptr published → 0x{:08X}", PLATFORM_PTR_ADDR);
    log::info!("[stage1] jumping to kernel...");
    log::info!("================================");

    // ── Call boot_main directly (proves the same code path as real kernel) ───
    boot_main(&platform)
}
