#[cfg(feature = "board-cyd")]
use common::*;

#[cfg(feature = "board-cyd")]
const BOOTROM_OFFSET: u32 = {
    match u32::from_str_radix(env!("BOOTROM_OFFSET"), 10) {
        Ok(v) => v,
        Err(_) => panic!("BOOTROM_OFFSET not a valid u32"),
    }
};
#[cfg(feature = "board-cyd")]
const BOOTROM_SIZE: u32 = {
    match u32::from_str_radix(env!("BOOTROM_SIZE"), 10) {
        Ok(v) => v,
        Err(_) => panic!("BOOTROM_SIZE not a valid u32"),
    }
};
#[cfg(feature = "board-cyd")]
#[allow(dead_code)]
const NVS_OFFSET: u32 = {
    match u32::from_str_radix(env!("NVS_OFFSET"), 10) {
        Ok(v) => v,
        Err(_) => panic!("NVS_OFFSET not a valid u32"),
    }
};

#[cfg(feature = "board-cyd")]
pub fn cyd_boot() -> ! {
    use esp_idf_svc::hal::peripherals::Peripherals;
    use esp_idf_svc::sys as idf;

    // ── Recovery mode check ───────────────────────────────────────────────────
    // BOOT button (IO0) is active LOW with internal pull-up.
    // Read GPIO0 via raw esp-idf before taking Peripherals so we don't
    // partially move the Peripherals struct.
    let recovery = unsafe {
        // Configure IO0 as input with pull-up, then sample.
        idf::gpio_config(&idf::gpio_config_t {
            pin_bit_mask: 1 << 0,
            mode: idf::gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: idf::gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: idf::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: idf::gpio_int_type_t_GPIO_INTR_DISABLE,
        });
        idf::gpio_get_level(0) == 0 // active low
    };

    let peripherals = Peripherals::take().expect("Peripherals already taken");

    // Initialise all CYD peripherals.
    let platform = hal_cyd::CydPlatform::new(peripherals);

    if recovery {
        log::info!("[stage1] BOOT button held — entering recovery mode");
        common::recovery_main(&platform);
        // recovery_main returns ! so we never reach here
    }

    log::info!("[stage1] CYD boot — checking SD card");
    if hw::sd_init() {
        log::info!("[stage1] SD card ready — loading flashpoint.rom");
        // NOTE (Plan 05): sd_read_rom must DMA the full ROM into DRAM at
        // sd_load_addr() before returning. The buf here is a scratch space
        // for header validation only; the actual jump target is sd_load_addr().
        let mut buf = [0u8; HEADER_V1_SIZE + 512];
        if hw::sd_read_rom(&mut buf).is_some() {
            match super::helpers::try_boot_from_buffer(&buf) {
                Ok(entry) => {
                    log::info!("[stage1] SD ROM valid — jumping to 0x{:08X}", entry);
                    hw::publish_platform_ptr(core::ptr::null());
                    hw::jump_to(entry);
                }
                Err(e) => {
                    log::warn!("[stage1] SD ROM rejected ({:?}) — trying internal flash", e);
                }
            }
        } else {
            log::warn!("[stage1] SD read failed — trying internal flash");
        }
    } else {
        log::info!("[stage1] no SD card — checking internal flash");
    }

    if BOOTROM_SIZE == 0 {
        log::warn!("[stage1] no internal boot ROM — entering recovery");
        common::recovery_main(&platform);
    }

    log::info!(
        "[stage1] reading internal ROM at offset=0x{:08X} size=0x{:08X}",
        BOOTROM_OFFSET,
        BOOTROM_SIZE
    );
    let mut hdr_buf = [0u8; HEADER_V1_SIZE];
    hw::flash_read(BOOTROM_OFFSET, &mut hdr_buf);

    match validate_header(
        &hdr_buf,
        crate::DEVICE_FEATURES,
        PLATFORM_ESP32,
        FLASHPOINT_CURRENT,
        FLASHPOINT_LAST_BREAKING,
    ) {
        Ok(_) => {
            let entry = super::helpers::flash_xip_addr(BOOTROM_OFFSET) + HEADER_V1_SIZE as u32;
            log::info!("[stage1] internal ROM valid — jumping to 0x{:08X}", entry);
            hw::publish_platform_ptr(core::ptr::null());
            hw::jump_to(entry);
        }
        Err(e) => {
            log::error!("[stage1] ROM header invalid ({:?}) — entering recovery", e);
            common::recovery_main(&platform);
        }
    }
}

#[cfg(feature = "board-cyd")]
#[allow(dead_code)]
mod hw {
    pub fn sd_init() -> bool {
        false
    }
    pub fn sd_read_rom(buf: &mut [u8]) -> Option<usize> {
        let _ = buf;
        None
    }
    pub fn flash_read(offset: u32, buf: &mut [u8]) {
        let _ = (offset, buf);
    }
    pub fn jump_to(addr: u32) -> ! {
        let _ = addr;
        loop {}
    }
    pub fn publish_platform_ptr(_ptr: *const ()) {}

    pub fn error_led(code: ErrorCode) -> ! {
        let _ = code;
        // Yield to FreeRTOS to keep IDLE tasks alive (avoids WDT spam).
        // Plan 05 will replace this with RGB LED blink + display error.
        loop {
            esp_idf_svc::hal::delay::FreeRtos::delay_ms(1000);
        }
    }

    #[derive(Clone, Copy)]
    pub enum ErrorCode {
        NoBoot,
        BadMagic,
        FeatureMismatch,
        BadChecksum,
    }
}
