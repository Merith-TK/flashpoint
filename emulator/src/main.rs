use esp_idf_svc::log::EspLogger;

fn main() {
    EspLogger::initialize_default();

    log::info!("================================");
    log::info!("  FLASHPOINT  v0.1.0");
    log::info!("  stage1 / boot-rom stub");
    log::info!("================================");
    log::info!("[STAGE1] platform  : ESP32");
    log::info!("[STAGE1] features  : display_tft | input_touch");
    log::info!("[STAGE1] SD card   : not present");
    log::info!("[STAGE1] internal  : boot-rom stub (embedded)");
    log::info!("[STAGE1] validating header...");
    log::info!("[STAGE1] header OK");
    log::info!("[STAGE1] jumping to boot-rom...");
    log::info!("================================");
    log::info!("  FLASHPOINT  OK");
    log::info!("================================");
    log::info!("system ready.");

    loop {
        unsafe { core::hint::spin_loop() }
    }
}
