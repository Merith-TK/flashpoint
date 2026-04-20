// firmware — the Flash-ROM burned to the device.
// Select board at build time:
//   (default)                            → board-cyd (CYD hardware)
//   --no-default-features --features board-qemu → QEMU emulator

pub mod hal;
mod stage1;

// CYD (ESP32-2432S028R) device capabilities.
// QEMU mirrors these for test parity.
pub const DEVICE_FEATURES: u64 = common::FEAT_DISP_TFT | common::FEAT_INPUT_TOUCH;

#[cfg(feature = "board-cyd")]
fn main() {
    use esp_idf_svc::log::EspLogger;
    EspLogger::initialize_default();
    log::info!("================================");
    log::info!("  FLASHPOINT  v0.1.0  [CYD]");
    log::info!("================================");
    stage1::stage1_main()
}

#[cfg(feature = "board-qemu")]
fn main() {
    use esp_idf_svc::log::EspLogger;
    EspLogger::initialize_default();
    log::info!("================================");
    log::info!("  FLASHPOINT  v0.1.0  [QEMU]");
    log::info!("================================");
    stage1::stage1_main()
}
