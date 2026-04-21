display_rs = """use common::{FrameBuffer, PlatformError};

pub fn clear() -> Result<(), PlatformError> {
    log::info!("[display] clear");
    Ok(())
}

pub fn flush(buf: &FrameBuffer) -> Result<(), PlatformError> {
    if buf.y % 60 == 0 {
        log::info!("[display] scanline y={}", buf.y);
    }
    Ok(())
}

pub fn width() -> u16 { 320 }
pub fn height() -> u16 { 240 }
"""

input_rs = """use common::Event;

pub fn poll_event() -> Option<Event> {
    None
}
"""

system_rs = """use common::{ChipId, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING};

pub fn battery_percent() -> u8 { 100 }
pub fn chip_id() -> ChipId { ChipId::Esp32 }
pub fn sleep_ms(ms: u32) {
    use esp_idf_svc::hal::delay::FreeRtos;
    FreeRtos::delay_ms(ms);
}
pub fn reboot() -> ! {
    panic!("reboot requested in emulator");
}
pub fn flashpoint_version() -> (u32, u32) {
    (FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING)
}
pub fn wasm_arena_limit() -> usize { 256 * 1024 }
pub fn lua_heap_limit() -> usize { 64 * 1024 }
"""

storage_rs = """use common::PlatformError;
use std::vec::Vec;

pub fn sd_read_sectors(_: u32, _: &mut [u8]) -> Result<(), PlatformError> {
    Err(PlatformError::SdReadError)
}
pub fn sd_write_sectors(_: u32, _: &[u8]) -> Result<(), PlatformError> {
    Err(PlatformError::SdWriteError)
}
pub fn sd_sector_count() -> u32 { 0 }
pub fn nvs_read(_: &str, _: &str) -> Result<Vec<u8>, PlatformError> {
    Err(PlatformError::NvsError)
}
pub fn nvs_write(_: &str, _: &str, _: &[u8]) -> Result<(), PlatformError> {
    Err(PlatformError::NvsError)
}
pub fn nvs_delete(_: &str, _: &str) -> Result<(), PlatformError> {
    Err(PlatformError::NvsError)
}
"""

lib_rs = """// hal-qemu — EmulatorPlatform for QEMU (board-qemu feature).

mod display;
mod input;
mod storage;
mod system;

use common::{
    ChipId, Event, FrameBuffer, Platform, PlatformError, FEAT_DISP_TFT,
};
use std::vec::Vec;

pub struct EmulatorPlatform;

impl EmulatorPlatform {
    pub fn new() -> Self { EmulatorPlatform }
}

impl Default for EmulatorPlatform {
    fn default() -> Self { Self::new() }
}

impl Platform for EmulatorPlatform {
    fn display_clear(&self) -> Result<(), PlatformError> { display::clear() }
    fn display_flush(&self, buf: &FrameBuffer) -> Result<(), PlatformError> { display::flush(buf) }
    fn display_width(&self) -> u16 { display::width() }
    fn display_height(&self) -> u16 { display::height() }
    fn poll_event(&self) -> Option<Event> { input::poll_event() }
    fn battery_percent(&self) -> u8 { system::battery_percent() }
    fn chip_id(&self) -> ChipId { system::chip_id() }
    fn sleep_ms(&self, ms: u32) { system::sleep_ms(ms) }
    fn reboot(&self) -> ! { system::reboot() }
    fn flashpoint_version(&self) -> (u32, u32) { system::flashpoint_version() }
    fn wasm_arena_limit(&self) -> usize { system::wasm_arena_limit() }
    fn lua_heap_limit(&self) -> usize { system::lua_heap_limit() }
    fn features(&self) -> u64 { FEAT_DISP_TFT }
    fn sd_read_sectors(&self, start: u32, buf: &mut [u8]) -> Result<(), PlatformError> { storage::sd_read_sectors(start, buf) }
    fn sd_write_sectors(&self, start: u32, buf: &[u8]) -> Result<(), PlatformError> { storage::sd_write_sectors(start, buf) }
    fn sd_sector_count(&self) -> u32 { storage::sd_sector_count() }
    fn nvs_read(&self, ns: &str, key: &str) -> Result<Vec<u8>, PlatformError> { storage::nvs_read(ns, key) }
    fn nvs_write(&self, ns: &str, key: &str, val: &[u8]) -> Result<(), PlatformError> { storage::nvs_write(ns, key, val) }
    fn nvs_delete(&self, ns: &str, key: &str) -> Result<(), PlatformError> { storage::nvs_delete(ns, key) }
}
"""

with open('src/display.rs', 'w') as f: f.write(display_rs)
with open('src/input.rs', 'w') as f: f.write(input_rs)
with open('src/system.rs', 'w') as f: f.write(system_rs)
with open('src/storage.rs', 'w') as f: f.write(storage_rs)
with open('src/lib.rs', 'w') as f: f.write(lib_rs)

print("QEMU refactored")
