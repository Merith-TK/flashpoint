// Stage 1 — Flashpoint chainload loader
//
// Dispatches to the correct boot path based on the board feature:
//   board-qemu  → validate embedded ROM → call common::boot_main directly
//   board-cyd   → try SD card → fallback to internal flash → LED error

use common::*;

// ── Compile-time flash layout (board-cyd, from build.rs) ─────────────────────

const BOOTROM_OFFSET: u32 = {
    match u32::from_str_radix(env!("BOOTROM_OFFSET"), 10) {
        Ok(v) => v,
        Err(_) => panic!("BOOTROM_OFFSET not a valid u32"),
    }
};
const BOOTROM_SIZE: u32 = {
    match u32::from_str_radix(env!("BOOTROM_SIZE"), 10) {
        Ok(v) => v,
        Err(_) => panic!("BOOTROM_SIZE not a valid u32"),
    }
};
#[allow(dead_code)]
const NVS_OFFSET: u32 = {
    match u32::from_str_radix(env!("NVS_OFFSET"), 10) {
        Ok(v) => v,
        Err(_) => panic!("NVS_OFFSET not a valid u32"),
    }
};

// ── ROM embedded at compile time (board-qemu, from build.rs) ─────────────────

#[cfg(feature = "board-qemu")]
static EMBEDDED_ROM: &[u8] = include_bytes!(env!("FLASHPOINT_ROM_PATH"));

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn stage1_main() -> ! {
    #[cfg(feature = "board-qemu")]
    { qemu_boot() }

    #[cfg(feature = "board-cyd")]
    { cyd_boot() }

    // Compile error if neither board feature is active (non-test builds only).
    #[cfg(all(not(test), not(any(feature = "board-qemu", feature = "board-cyd"))))]
    core::compile_error!("firmware requires --features board-cyd or --features board-qemu");

    // Satisfies the `-> !` return type in test builds with no board feature.
    #[allow(unreachable_code)]
    loop {}
}

// ── QEMU boot path ────────────────────────────────────────────────────────────

#[cfg(feature = "board-qemu")]
fn qemu_boot() -> ! {
    let payload_offset = match validate_header(
        EMBEDDED_ROM,
        crate::DEVICE_FEATURES,
        PLATFORM_ESP32,
        FLASHPOINT_CURRENT,
        FLASHPOINT_LAST_BREAKING,
    ) {
        Ok(offset) => {
            log::info!("[stage1] header OK — payload at offset {}", offset);
            offset
        }
        Err(e) => {
            log::error!("[stage1] header validation failed: {:?}", e);
            log::error!("[stage1] rebuild with FLASHPOINT_ROM set for full E2E");
            loop {}
        }
    };

    let payload_len = u32::from_le_bytes(
        EMBEDDED_ROM[OFF_PAYLOAD_LEN..OFF_PAYLOAD_LEN + 4].try_into().unwrap()
    ) as usize;
    if let Err(e) = verify_crc32(
        &EMBEDDED_ROM[..payload_offset],
        &EMBEDDED_ROM[payload_offset..payload_offset + payload_len],
    ) {
        log::error!("[stage1] checksum verification failed: {:?}", e);
        loop {}
    }
    log::info!("[stage1] checksum OK");

    let platform = crate::hal::ActivePlatform::new();

    // QEMU: boot_main is called directly — no cross-binary jump, no ptr write needed.
    // Real hardware (Plan 06): write fat-ptr to PLATFORM_PTR_ADDR then jump to kernel.
    // NOTE: PLATFORM_PTR_ADDR must be verified against the FreeRTOS heap layout before
    // enabling the real-hardware path — 0x3FFB_0000 overlaps heap in current ESP-IDF config.
    log::info!("[stage1] jumping to kernel...");
    log::info!("================================");

    common::boot_main(&platform)
}

// ── CYD boot path ─────────────────────────────────────────────────────────────

#[cfg(feature = "board-cyd")]
fn cyd_boot() -> ! {
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
        idf::gpio_get_level(0) == 0  // active low
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
            match try_boot_from_buffer(&buf) {
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

    log::info!("[stage1] reading internal ROM at offset=0x{:08X} size=0x{:08X}",
        BOOTROM_OFFSET, BOOTROM_SIZE);
    let mut hdr_buf = [0u8; HEADER_V1_SIZE];
    hw::flash_read(BOOTROM_OFFSET, &mut hdr_buf);

    match validate_header(&hdr_buf, crate::DEVICE_FEATURES, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING) {
        Ok(_) => {
            let entry = flash_xip_addr(BOOTROM_OFFSET) + HEADER_V1_SIZE as u32;
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

// ── Hardware stubs (board-cyd, replaced in Plan 05) ───────────────────────────

#[cfg(feature = "board-cyd")]
#[allow(dead_code)]
mod hw {
    pub fn sd_init() -> bool { false }
    pub fn sd_read_rom(buf: &mut [u8]) -> Option<usize> { let _ = buf; None }
    pub fn flash_read(offset: u32, buf: &mut [u8]) { let _ = (offset, buf); }
    pub fn jump_to(addr: u32) -> ! { let _ = addr; loop {} }
    pub fn publish_platform_ptr(_ptr: *const ()) {}

    pub fn error_led(code: ErrorCode) -> ! {
        let _ = code;
        // Yield to FreeRTOS to keep IDLE tasks alive (avoids WDT spam).
        // Plan 05 will replace this with RGB LED blink + display error.
        loop { esp_idf_svc::hal::delay::FreeRtos::delay_ms(1000); }
    }

    #[derive(Clone, Copy)]
    pub enum ErrorCode { NoBoot, BadMagic, FeatureMismatch, BadChecksum }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn try_boot_from_buffer(buf: &[u8]) -> Result<u32, HeaderError> {
    let offset = validate_header(buf, crate::DEVICE_FEATURES, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING)?;
    Ok(sd_load_addr() + offset as u32)
}

fn flash_xip_addr(offset: u32) -> u32 { 0x400C_0000 + offset }
fn sd_load_addr()               -> u32 { 0x3FFB_8000 }

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use common::{build_header, PayloadType};

    #[test]
    fn layout_constants_parse() {
        let _ = BOOTROM_OFFSET;
        let _ = BOOTROM_SIZE;
        let _ = NVS_OFFSET;
    }

    #[test]
    fn try_boot_rejects_bad_magic() {
        let buf = [0u8; HEADER_V1_SIZE];
        assert!(try_boot_from_buffer(&buf).is_err());
    }

    #[test]
    fn try_boot_accepts_valid_header() {
        let hdr = build_header(PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, 0, 64, PayloadType::Native, "", [0, 0, 0], 0);
        assert!(try_boot_from_buffer(&hdr).is_ok());
    }

    #[test]
    fn try_boot_rejects_feature_mismatch() {
        let hdr = build_header(PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, FEAT_PSRAM, 64, PayloadType::Native, "", [0, 0, 0], 0);
        assert_eq!(try_boot_from_buffer(&hdr), Err(HeaderError::MissingFeatures));
    }
}
