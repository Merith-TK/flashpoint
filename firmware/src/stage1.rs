// Stage 1 — Flashpoint chainload loader
//
// Reads SD card first, falls back to internal flash.
// Validates the ROM header, publishes the Platform vtable pointer,
// and jumps to the kernel entry point.

use flashpoint_common::*;

// ─── Compile-time flash layout (from build.rs) ───────────────────────────────

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

// ─── Hardware stubs (replaced in step 0.5) ───────────────────────────────────

mod hw {
    pub fn sd_init() -> bool { false }

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
        loop {}
    }

    #[derive(Clone, Copy)]
    pub enum ErrorCode {
        NoBoot,
        BadMagic,
        FeatureMismatch,
        BadChecksum,
    }
}

// ─── Boot logic ──────────────────────────────────────────────────────────────

pub fn stage1_main() -> ! {
    if hw::sd_init() {
        let mut buf = [0u8; HEADER_V1_SIZE + 512];
        if let Some(_) = hw::sd_read_rom(&mut buf) {
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

    match validate_header(&hdr_buf, device_features(), PLATFORM_ESP32) {
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

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn try_boot_from_buffer(buf: &[u8]) -> Result<u32, HeaderError> {
    let payload_offset = validate_header(buf, device_features(), PLATFORM_ESP32)?;
    Ok(sd_load_addr() + payload_offset as u32)
}

fn device_features() -> u64 { 0 } // TODO (step 0.5): read from capabilities module

fn flash_xip_addr(offset: u32) -> u32 { 0x400C_0000 + offset }

fn sd_load_addr() -> u32 { 0x3FFB_8000 }

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use flashpoint_common::build_header;

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
        let hdr = build_header(PLATFORM_ESP32, [0, 1, 0], 0, 0, 64, dummy_checksum());
        assert!(try_boot_from_buffer(&hdr).is_ok());
    }

    #[test]
    fn try_boot_rejects_feature_mismatch() {
        let hdr = build_header(PLATFORM_ESP32, [0, 1, 0], 0, FEAT_PSRAM, 64, dummy_checksum());
        assert_eq!(try_boot_from_buffer(&hdr), Err(HeaderError::MissingFeatures));
    }
}
