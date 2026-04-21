#[cfg(feature = "board-qemu")]
use common::*;

#[cfg(feature = "board-qemu")]
static EMBEDDED_ROM: &[u8] = include_bytes!(env!("FLASHPOINT_ROM_PATH"));

#[cfg(feature = "board-qemu")]
pub fn qemu_boot() -> ! {
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
        EMBEDDED_ROM[OFF_PAYLOAD_LEN..OFF_PAYLOAD_LEN + 4]
            .try_into()
            .unwrap(),
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
