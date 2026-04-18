// CYD (ESP32-2432S028R) Platform implementation
//
// Pins (to be verified against schematic in step 0.5):
//   LCD SPI:   MOSI=13, MISO=12, CLK=14, CS=15, DC=2, BL=21
//   Touch SPI: MOSI=32, MISO=39, CLK=25, CS=33, IRQ=36
//   SD SPI:    MOSI=23, MISO=19, CLK=18, CS=5
//   RGB LED:   R=4, G=16, B=17
//
// Everything here is stubbed with todo!() until step 0.5.

use super::platform::{FrameBuffer, Platform, PlatformError};
use common::{ChipId, Event};

pub struct CydPlatform {
    // TODO (step 0.5): hold SPI bus handles, display driver, touch driver, SD handle, NVS
}

impl CydPlatform {
    pub fn new() -> Self {
        // TODO (step 0.5): init peripherals, SPI buses, drivers
        CydPlatform {}
    }
}

impl Platform for CydPlatform {
    fn sd_read_sectors(&self, _start: u32, _buf: &mut [u8]) -> Result<(), PlatformError> {
        todo!("step 0.5: SD SPI init + embedded-sdmmc read")
    }

    fn sd_write_sectors(&self, _start: u32, _buf: &[u8]) -> Result<(), PlatformError> {
        todo!("step 0.5: SD SPI write")
    }

    fn sd_sector_count(&self) -> u32 {
        todo!("step 0.5: SD capacity query")
    }

    fn nvs_read(&self, _ns: &str, _key: &str) -> Result<alloc::vec::Vec<u8>, PlatformError> {
        todo!("step 0.5: esp-idf-sys nvs_get_blob")
    }

    fn nvs_write(&self, _ns: &str, _key: &str, _val: &[u8]) -> Result<(), PlatformError> {
        todo!("step 0.5: esp-idf-sys nvs_set_blob")
    }

    fn nvs_delete(&self, _ns: &str, _key: &str) -> Result<(), PlatformError> {
        todo!("step 0.5: esp-idf-sys nvs_erase_key")
    }

    fn display_flush(&self, _buf: &FrameBuffer) -> Result<(), PlatformError> {
        todo!("step 0.5: mipidsi ILI9341 set_pixels for scanline")
    }

    fn display_clear(&self) -> Result<(), PlatformError> {
        todo!("step 0.5: mipidsi clear()")
    }

    fn display_width(&self) -> u16  { 320 }
    fn display_height(&self) -> u16 { 240 }

    fn poll_event(&self) -> Option<Event> {
        todo!("step 0.5: XPT2046 read + zone mapping")
    }

    fn battery_percent(&self) -> u8 {
        100 // CYD has no battery — stub returns full
    }

    fn chip_id(&self) -> ChipId { ChipId::Esp32 }

    fn reboot(&self) -> ! {
        todo!("step 0.5: esp_restart()")
    }

    fn sleep_ms(&self, _ms: u32) {
        todo!("step 0.5: vTaskDelay or thread::sleep")
    }
}

extern crate alloc;
