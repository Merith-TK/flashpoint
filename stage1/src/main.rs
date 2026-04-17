// Stage 1 — Flashpoint chainload loader
//
// Lives in internal flash. Burned once. Job: find a valid boot-rom
// (SD card first, internal flash second), check feature compatibility,
// hand the Platform vtable to the boot-rom, and jump.
//
// This file contains the hardware-agnostic logic. Hardware-specific
// init (SDMMC, SPI, LED) is gated behind the `hardware` module which
// is implemented in step 0.5.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use flashpoint_common::*;

// ─── Compile-time flash layout (from build.rs) ───────────────────────────────

const BOOTROM_OFFSET: u32 = {
    // SAFETY: build.rs always emits these; unwrap is compile-time.
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
const NVS_OFFSET: u32 = {
    match u32::from_str_radix(env!("NVS_OFFSET"), 10) {
        Ok(v) => v,
        Err(_) => panic!("NVS_OFFSET not a valid u32"),
    }
};

// ─── Hardware stubs (replaced in step 0.5) ───────────────────────────────────

mod hw {
    /// Initialise SDMMC peripheral and mount FAT32 partition.
    /// Returns true on success.
    pub fn sd_init() -> bool {
        // TODO (step 0.5): init SPI-mode SD + embedded-sdmmc FatFS mount
        false
    }

    /// Read `flashpoint.rom` from the SD FAT32 partition into `buf`.
    /// Returns number of bytes read, or None if file not found / error.
    pub fn sd_read_rom(buf: &mut [u8]) -> Option<usize> {
        // TODO (step 0.5): open "flashpoint.rom", read header + payload
        let _ = buf;
        None
    }

    /// Read bytes from internal flash at `offset` into `buf`.
    pub fn flash_read(offset: u32, buf: &mut [u8]) {
        // TODO (step 0.5): esp_flash_read or direct memory map
        let _ = (offset, buf);
    }

    /// Jump to the boot-rom entry point at `addr`.
    /// Never returns.
    pub fn jump_to(addr: u32) -> ! {
        // TODO (step 0.5): function pointer cast + call
        let _ = addr;
        loop {}
    }

    /// Write the platform fat-pointer to the agreed handoff address.
    pub fn publish_platform_ptr(_ptr: *const ()) {
        // TODO (step 0.5): write to PLATFORM_PTR_ADDR
    }

    /// Signal an error via RGB LED blink pattern.
    pub fn error_led(code: ErrorCode) -> ! {
        // TODO (step 0.5): GPIO blink patterns
        let _ = code;
        loop {}
    }

    #[derive(Clone, Copy)]
    pub enum ErrorCode {
        NoBoot,         // solid red — nothing to boot
        BadMagic,       // fast red blink
        FeatureMismatch,// slow red + blue
        BadChecksum,    // red + green
    }
}

// ─── Core boot logic (hardware-agnostic) ─────────────────────────────────────

#[cfg_attr(not(test), no_mangle)]
pub fn stage1_main() -> ! {
    // 1. Attempt SD boot
    if hw::sd_init() {
        let mut buf = [0u8; HEADER_V1_SIZE + 512]; // header + enough for a read probe
        if let Some(_) = hw::sd_read_rom(&mut buf) {
            match try_boot_from_buffer(&buf) {
                Ok(entry) => {
                    hw::publish_platform_ptr(core::ptr::null()); // TODO: real Platform ptr
                    hw::jump_to(entry);
                }
                Err(e) => {
                    // Log error, fall through to internal
                    let _ = e;
                }
            }
        }
    }

    // 2. Attempt internal flash boot
    if BOOTROM_SIZE == 0 {
        hw::error_led(hw::ErrorCode::NoBoot);
    }

    let mut hdr_buf = [0u8; HEADER_V1_SIZE];
    hw::flash_read(BOOTROM_OFFSET, &mut hdr_buf);

    match validate_header(&hdr_buf, device_features(), PLATFORM_ESP32) {
        Ok(_) => {
            // XIP: entry point is directly in flash address space
            let entry = flash_xip_addr(BOOTROM_OFFSET) + HEADER_V1_SIZE as u32;
            hw::publish_platform_ptr(core::ptr::null()); // TODO: real Platform ptr
            hw::jump_to(entry);
        }
        Err(HeaderError::MissingFeatures) => hw::error_led(hw::ErrorCode::FeatureMismatch),
        Err(HeaderError::BadChecksum)     => hw::error_led(hw::ErrorCode::BadChecksum),
        Err(_)                            => hw::error_led(hw::ErrorCode::BadMagic),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Attempt to validate and locate an entry point from a header+payload buffer.
/// Returns the load address of the entry point on success.
fn try_boot_from_buffer(buf: &[u8]) -> Result<u32, HeaderError> {
    // Validate header fields (checksum validated separately once full payload loaded)
    let payload_offset = validate_header(buf, device_features(), PLATFORM_ESP32)?;
    // For SD boot: payload is loaded into SRAM starting at a fixed load address
    Ok(sd_load_addr() + payload_offset as u32)
}

/// Published DEVICE_FEATURES bitmask for this build.
/// Replaced per-board in step 0.5 via flash-rom/src/capabilities.rs.
fn device_features() -> u64 {
    // TODO (step 0.5): read from flash-rom capabilities module
    0
}

/// Convert a flash byte offset to the ESP32 XIP memory-mapped address.
/// ESP32 SPI flash is mapped at 0x400C_0000 for direct execution.
fn flash_xip_addr(offset: u32) -> u32 {
    0x400C_0000 + offset
}

/// SRAM address where SD-loaded boot-rom payloads are copied.
/// Must be above Stage 1's own stack. 0x3FFB_8000 is safely above stack region.
fn sd_load_addr() -> u32 {
    0x3FFB_8000
}

// ─── no_std entry (step 0.5 will wire this to esp-idf-sys) ──────────────────

#[cfg(not(test))]
#[no_mangle]
extern "C" fn app_main() {
    stage1_main()
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use flashpoint_common::build_header;

    fn dummy_checksum() -> [u8; 32] { [0; 32] }

    #[test]
    fn layout_constants_no_bootrom() {
        // Without BOOTROM_BIN, build.rs emits BOOTROM_SIZE=0, NVS_OFFSET=65536
        // This test just confirms the constants parse correctly.
        let _ = BOOTROM_OFFSET;
        let _ = BOOTROM_SIZE;
        let _ = NVS_OFFSET;
    }

    #[test]
    fn try_boot_rejects_bad_magic() {
        let mut buf = [0u8; HEADER_V1_SIZE];
        // leave magic as zeros — should fail BadMagic
        let result = try_boot_from_buffer(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn try_boot_accepts_valid_header_no_features() {
        let hdr = build_header(PLATFORM_ESP32, [0,1,0], 0, 0, 64, dummy_checksum());
        let result = try_boot_from_buffer(&hdr);
        // Succeeds: no required features, header valid
        assert!(result.is_ok());
    }

    #[test]
    fn try_boot_rejects_feature_mismatch() {
        let hdr = build_header(PLATFORM_ESP32, [0,1,0], 0, FEAT_PSRAM, 64, dummy_checksum());
        let result = try_boot_from_buffer(&hdr);
        assert_eq!(result, Err(HeaderError::MissingFeatures));
    }
}
