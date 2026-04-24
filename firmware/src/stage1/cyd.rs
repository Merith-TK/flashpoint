#[cfg(feature = "board-cyd")]
pub fn cyd_boot() -> ! {
    use common::Platform;
    use esp_idf_svc::hal::peripherals::Peripherals;
    use esp_idf_svc::sys as idf;

    let recovery_held = unsafe {
        // Configure IO0 as input with pull-up, then sample (active low).
        idf::gpio_config(&idf::gpio_config_t {
            pin_bit_mask: 1 << 0,
            mode: idf::gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: idf::gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: idf::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: idf::gpio_int_type_t_GPIO_INTR_DISABLE,
        });
        idf::gpio_get_level(0) == 0
    };

    let peripherals = Peripherals::take().expect("Peripherals already taken");
    let platform = hal_cyd::CydPlatform::new(peripherals);

    if recovery_held {
        log::info!("[stage1] BOOT button held — entering recovery mode");
        common::recovery_main(&platform);
    }

    // ── SD card is required ───────────────────────────────────────────────────
    let fs = match crate::fs::mount(&platform) {
        Ok(fs) => fs,
        Err(e) => {
            log::error!("[stage1] no SD card ({:?}) — SD is required for operation", e);
            common::recovery_main_with_status(&platform, "NO SD CARD - INSERT AND REBOOT");
        }
    };

    // Wrap platform so nvs_* calls route to SD-backed tinykv stores.
    let sd_plat = crate::sd_platform::SdPlatform::new(&platform, fs);

    // ── Determine boot mode (default: wasm) ───────────────────────────────────
    let mode = sd_plat
        .nvs_read("flashpoint", "boot-mode")
        .ok()
        .and_then(|b| core::str::from_utf8(&b).ok().map(|s| s.trim().to_owned()))
        .unwrap_or_else(|| "wasm".into());

    log::info!("[stage1] boot mode = {}", mode);

    match mode.as_str() {
        "lua" => crate::runtime::lua::boot(&sd_plat),
        _ => crate::runtime::wasm::boot(&sd_plat),
    }
}

