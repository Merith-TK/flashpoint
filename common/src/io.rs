use crate::error::PlatformError;
use crate::types::{ChipId, Event, FrameBuffer};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(target_os = "espidf")]
pub fn esp_idf_uart_poll_byte() -> Option<u8> {
    extern "C" {
        fn read(fd: i32, buf: *mut core::ffi::c_void, count: usize) -> isize;
        fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
    }
    const F_GETFL: i32 = 3;
    const F_SETFL: i32 = 4;
    const O_NONBLOCK: i32 = 0x4000;

    unsafe {
        let flags = fcntl(0, F_GETFL, 0);
        fcntl(0, F_SETFL, flags | O_NONBLOCK);
        let mut byte: u8 = 0;
        let n = read(0, &mut byte as *mut u8 as *mut core::ffi::c_void, 1);
        fcntl(0, F_SETFL, flags);
        if n == 1 {
            Some(byte)
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "espidf"))]
pub fn esp_idf_uart_poll_byte() -> Option<u8> {
    None
}

pub trait Platform {
    fn sd_read_sectors(&self, _start: u32, _buf: &mut [u8]) -> Result<(), PlatformError> {
        log::warn!("sd_read_sectors not supported on this device");
        Err(PlatformError::NotSupported)
    }
    fn sd_write_sectors(&self, _start: u32, _buf: &[u8]) -> Result<(), PlatformError> {
        log::warn!("sd_write_sectors not supported on this device");
        Err(PlatformError::NotSupported)
    }
    fn sd_sector_count(&self) -> u32 {
        0
    }
    fn nvs_read(&self, _ns: &str, _key: &str) -> Result<Vec<u8>, PlatformError> {
        log::warn!("nvs_read not supported on this device");
        Err(PlatformError::NotSupported)
    }
    fn nvs_write(&self, _ns: &str, _key: &str, _val: &[u8]) -> Result<(), PlatformError> {
        log::warn!("nvs_write not supported on this device");
        Err(PlatformError::NotSupported)
    }
    fn nvs_delete(&self, _ns: &str, _key: &str) -> Result<(), PlatformError> {
        log::warn!("nvs_delete not supported on this device");
        Err(PlatformError::NotSupported)
    }

    fn display_flush(&self, _buf: &FrameBuffer) -> Result<(), PlatformError> {
        log::warn!("display_flush not supported on this device");
        Err(PlatformError::NotSupported)
    }
    fn display_clear(&self) -> Result<(), PlatformError> {
        log::warn!("display_clear not supported on this device");
        Err(PlatformError::NotSupported)
    }
    fn display_width(&self) -> u16 {
        0
    }
    fn display_height(&self) -> u16 {
        0
    }

    fn poll_event(&self) -> Option<Event> {
        None
    }

    fn uart_poll_byte(&self) -> Option<u8> {
        esp_idf_uart_poll_byte()
    }

    fn poll_touch_xy(&self) -> Option<(u16, u16)> {
        None
    }

    fn led_rgb(&self, _r: u8, _g: u8, _b: u8) -> Result<(), PlatformError> {
        log::warn!("led_rgb not supported on this device");
        Err(PlatformError::NotSupported)
    }

    fn battery_percent(&self) -> u8 {
        100
    }
    fn chip_id(&self) -> ChipId {
        ChipId::Esp32
    }
    fn reboot(&self) -> ! {
        loop {}
    }
    fn sleep_ms(&self, _ms: u32) {}
    fn flashpoint_version(&self) -> (u32, u32) {
        (
            crate::header::FLASHPOINT_CURRENT,
            crate::header::FLASHPOINT_LAST_BREAKING,
        )
    }
    fn wasm_arena_limit(&self) -> usize {
        0
    }
    fn lua_heap_limit(&self) -> usize {
        0
    }
    fn features(&self) -> u64 {
        0
    }
}
