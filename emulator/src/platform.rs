use common::{
    ChipId, Event, FrameBuffer, Platform, PlatformError,
    FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING,
};

pub struct EmulatorPlatform;

impl Platform for EmulatorPlatform {
    fn display_clear(&self) -> Result<(), PlatformError> {
        log::info!("[display] clear");
        Ok(())
    }

    fn display_flush(&self, buf: &FrameBuffer) -> Result<(), PlatformError> {
        // Log every 60 scanlines — keeps QEMU output readable
        if buf.y % 60 == 0 {
            log::info!("[display] scanline y={}", buf.y);
        }
        Ok(())
    }

    fn display_width(&self)  -> u16 { 320 }
    fn display_height(&self) -> u16 { 240 }

    // No input in QEMU — boot_main loops until BtnSelect, which never arrives.
    // emu-run kills QEMU after seeing "FLASHPOINT OK" in the log.
    fn poll_event(&self) -> Option<Event> { None }

    fn battery_percent(&self) -> u8 { 100 }
    fn chip_id(&self) -> ChipId { ChipId::Esp32 }

    fn sleep_ms(&self, ms: u32) {
        use esp_idf_svc::hal::delay::FreeRtos;
        FreeRtos::delay_ms(ms);
    }

    fn reboot(&self) -> ! {
        // Never called in QEMU (poll_event always returns None), but required by trait.
        panic!("reboot requested in emulator");
    }

    fn flashpoint_version(&self) -> (u32, u32) {
        (FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING)
    }

    // SD / NVS: not present in emulator
    fn sd_read_sectors(&self, _: u32, _: &mut [u8]) -> Result<(), PlatformError> {
        Err(PlatformError::SdReadError)
    }
    fn sd_write_sectors(&self, _: u32, _: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::SdWriteError)
    }
    fn sd_sector_count(&self) -> u32 { 0 }
    fn nvs_read(&self, _: &str, _: &str) -> Result<Vec<u8>, PlatformError> {
        Err(PlatformError::NvsError)
    }
    fn nvs_write(&self, _: &str, _: &str, _: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::NvsError)
    }
    fn nvs_delete(&self, _: &str, _: &str) -> Result<(), PlatformError> {
        Err(PlatformError::NvsError)
    }
}
