// hal-qemu — EmulatorPlatform for QEMU (board-qemu feature).

mod display;
mod input;
mod storage;
mod system;

use common::{ChipId, Event, FrameBuffer, Platform, PlatformError, FEAT_DISP_TFT};
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
    fn display_clear(&self) -> Result<(), PlatformError> {
        display::clear()
    }
    fn display_flush(&self, buf: &FrameBuffer) -> Result<(), PlatformError> {
        display::flush(buf)
    }
    fn display_width(&self) -> u16 {
        display::width()
    }
    fn display_height(&self) -> u16 {
        display::height()
    }
    fn poll_event(&self) -> Option<Event> {
        input::poll_event()
    }
    fn battery_percent(&self) -> u8 {
        system::battery_percent()
    }
    fn chip_id(&self) -> ChipId {
        system::chip_id()
    }
    fn sleep_ms(&self, ms: u32) {
        system::sleep_ms(ms)
    }
    fn reboot(&self) -> ! {
        system::reboot()
    }
    fn flashpoint_version(&self) -> (u32, u32) {
        system::flashpoint_version()
    }
    fn wasm_arena_limit(&self) -> usize {
        system::wasm_arena_limit()
    }
    fn lua_heap_limit(&self) -> usize {
        system::lua_heap_limit()
    }
    fn features(&self) -> u64 {
        FEAT_DISP_TFT
    }
    fn sd_read_sectors(&self, start: u32, buf: &mut [u8]) -> Result<(), PlatformError> {
        storage::sd_read_sectors(start, buf)
    }
    fn sd_write_sectors(&self, start: u32, buf: &[u8]) -> Result<(), PlatformError> {
        storage::sd_write_sectors(start, buf)
    }
    fn sd_sector_count(&self) -> u32 {
        storage::sd_sector_count()
    }
    fn nvs_read(&self, ns: &str, key: &str) -> Result<Vec<u8>, PlatformError> {
        storage::nvs_read(ns, key)
    }
    fn nvs_write(&self, ns: &str, key: &str, val: &[u8]) -> Result<(), PlatformError> {
        storage::nvs_write(ns, key, val)
    }
    fn nvs_delete(&self, ns: &str, key: &str) -> Result<(), PlatformError> {
        storage::nvs_delete(ns, key)
    }
}
