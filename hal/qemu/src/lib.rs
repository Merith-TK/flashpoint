// hal-qemu — EmulatorPlatform for QEMU (board-qemu feature).
//
// All display output goes to UART via log::info!.
// Input always returns None — boot_main loops until BtnSelect, which never fires;
// emu-run kills QEMU after seeing the expected log output.

use common::{
    ChipId, Event, FrameBuffer, Platform, PlatformError, FEAT_DISP_TFT, FLASHPOINT_CURRENT,
    FLASHPOINT_LAST_BREAKING,
};
use std::vec::Vec;

pub struct EmulatorPlatform;

impl EmulatorPlatform {
    pub fn new() -> Self {
        EmulatorPlatform
    }
}

impl Default for EmulatorPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for EmulatorPlatform {
    // ── Display ───────────────────────────────────────────────────────────────
    fn display_clear(&self) -> Result<(), PlatformError> {
        log::info!("[display] clear");
        Ok(())
    }

    fn display_flush(&self, buf: &FrameBuffer) -> Result<(), PlatformError> {
        // Log every 60 scanlines to keep output readable
        if buf.y % 60 == 0 {
            log::info!("[display] scanline y={}", buf.y);
        }
        Ok(())
    }

    fn display_width(&self) -> u16 {
        320
    }
    fn display_height(&self) -> u16 {
        240
    }

    // ── Input ─────────────────────────────────────────────────────────────────
    fn poll_event(&self) -> Option<Event> {
        None
    }

    // ── System ────────────────────────────────────────────────────────────────
    fn battery_percent(&self) -> u8 {
        100
    }
    fn chip_id(&self) -> ChipId {
        ChipId::Esp32
    }

    fn sleep_ms(&self, ms: u32) {
        use esp_idf_svc::hal::delay::FreeRtos;
        FreeRtos::delay_ms(ms);
    }

    fn reboot(&self) -> ! {
        // Never reached in QEMU (poll_event always returns None)
        panic!("reboot requested in emulator");
    }

    fn flashpoint_version(&self) -> (u32, u32) {
        (FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING)
    }

    fn wasm_arena_limit(&self) -> usize {
        256 * 1024
    }
    fn lua_heap_limit(&self) -> usize {
        64 * 1024
    }

    // ── Capabilities ──────────────────────────────────────────────────────────
    fn features(&self) -> u64 {
        FEAT_DISP_TFT // QEMU has a simulated display only
    }

    // ── Storage: not available in QEMU ───────────────────────────────────────
    fn sd_read_sectors(&self, _: u32, _: &mut [u8]) -> Result<(), PlatformError> {
        Err(PlatformError::SdReadError)
    }
    fn sd_write_sectors(&self, _: u32, _: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::SdWriteError)
    }
    fn sd_sector_count(&self) -> u32 {
        0
    }
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
