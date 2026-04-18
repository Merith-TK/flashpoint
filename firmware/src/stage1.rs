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

    // Compile error if neither board feature is active — intentional.
    // One of the blocks above always diverges (-> !) when a valid feature is set.
    #[cfg(not(any(feature = "board-qemu", feature = "board-cyd")))]
    core::compile_error!("firmware requires --features board-cyd or --features board-qemu");
}

// ── QEMU boot path ────────────────────────────────────────────────────────────

#[cfg(feature = "board-qemu")]
fn qemu_boot() -> ! {
    match validate_header(
        EMBEDDED_ROM,
        crate::DEVICE_FEATURES,
        PLATFORM_ESP32,
        FLASHPOINT_CURRENT,
        FLASHPOINT_LAST_BREAKING,
    ) {
        Ok(payload_offset) => {
            log::info!("[stage1] header OK — payload at offset {}", payload_offset);
        }
        Err(e) => {
            log::error!("[stage1] header validation failed: {:?}", e);
            log::error!("[stage1] rebuild with FLASHPOINT_ROM set for full E2E");
            loop {}
        }
    }

    let platform = crate::hal::ActivePlatform::new();
    let platform_ref: &dyn Platform = &platform;
    let fat_ptr = &platform_ref as *const &dyn Platform as *const ();
    unsafe {
        core::ptr::write(PLATFORM_PTR_ADDR as *mut *const (), fat_ptr as *const ());
    }

    log::info!("[stage1] platform ptr → 0x{:08X}", PLATFORM_PTR_ADDR);
    log::info!("[stage1] jumping to kernel...");
    log::info!("================================");

    common::boot_main(&platform)
}

// ── CYD boot path ─────────────────────────────────────────────────────────────

#[cfg(feature = "board-cyd")]
fn cyd_boot() -> ! {
    if hw::sd_init() {
        let mut buf = [0u8; HEADER_V1_SIZE + 512];
        if hw::sd_read_rom(&mut buf).is_some() {
            match try_boot_from_buffer(&buf) {
                Ok(entry) => {
                    hw::publish_platform_ptr(core::ptr::null());
                    hw::jump_to(entry);
                }
                Err(e) => { let _ = e; }
            }
        }
    }

    if BOOTROM_SIZE == 0 {
        hw::error_led(hw::ErrorCode::NoBoot);
    }

    let mut hdr_buf = [0u8; HEADER_V1_SIZE];
    hw::flash_read(BOOTROM_OFFSET, &mut hdr_buf);

    match validate_header(&hdr_buf, crate::DEVICE_FEATURES, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING) {
        Ok(_) => {
            let entry = flash_xip_addr(BOOTROM_OFFSET) + HEADER_V1_SIZE as u32;
            hw::publish_platform_ptr(core::ptr::null());
            hw::jump_to(entry);
        }
        Err(HeaderError::MissingFeatures) => hw::error_led(hw::ErrorCode::FeatureMismatch),
        Err(HeaderError::BadChecksum)     => hw::error_led(hw::ErrorCode::BadChecksum),
        Err(_)                            => hw::error_led(hw::ErrorCode::BadMagic),
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
    pub fn error_led(code: ErrorCode) -> ! { let _ = code; loop {} }

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
    use common::build_header;

    fn dummy_checksum() -> [u8; 32] { [0; 32] }

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
        let hdr = build_header(PLATFORM_ESP32, [0, 1, 0], FLASHPOINT_CURRENT, 0, 0, 64, dummy_checksum());
        assert!(try_boot_from_buffer(&hdr).is_ok());
    }

    #[test]
    fn try_boot_rejects_feature_mismatch() {
        let hdr = build_header(PLATFORM_ESP32, [0, 1, 0], FLASHPOINT_CURRENT, 0, FEAT_PSRAM, 64, dummy_checksum());
        assert_eq!(try_boot_from_buffer(&hdr), Err(HeaderError::MissingFeatures));
    }
}
