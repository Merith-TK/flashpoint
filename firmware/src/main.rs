// firmware — the Flash-ROM burned to the device.
// Select board at build time:
//   (default)                            → board-cyd (CYD hardware)
//   --no-default-features --features board-qemu → QEMU emulator

// board-cyd: bare-metal no_std ESP-IDF app; board-qemu: std ESP-IDF via esp-idf-svc
#![cfg_attr(all(not(test), feature = "board-cyd"), no_std)]
#![cfg_attr(all(not(test), feature = "board-cyd"), no_main)]

#[cfg(feature = "board-cyd")]
extern crate alloc;

pub mod hal;
mod stage1;

// CYD (ESP32-2432S028R) device capabilities.
// QEMU mirrors these for test parity.
pub const DEVICE_FEATURES: u64 =
    common::FEAT_DISP_TFT |
    common::FEAT_INPUT_TOUCH;

// ── board-cyd entry point ─────────────────────────────────────────────────────
#[cfg(all(not(test), feature = "board-cyd"))]
#[no_mangle]
extern "C" fn app_main() {
    stage1::stage1_main()
}

// ── board-qemu entry point (via esp-idf-svc binstart) ────────────────────────
#[cfg(feature = "board-qemu")]
fn main() {
    use esp_idf_svc::log::EspLogger;
    EspLogger::initialize_default();
    log::info!("================================");
    log::info!("  FLASHPOINT  v0.1.0  [QEMU]");
    log::info!("================================");
    stage1::stage1_main()
}

// ── board-cyd panic handler ───────────────────────────────────────────────────
#[cfg(all(not(test), feature = "board-cyd"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
