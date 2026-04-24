/// SdPlatform — wraps a `&dyn Platform` and overrides `nvs_*` methods to use
/// SD-backed tinykv stores instead of ESP-IDF NVS flash.
///
/// All other Platform methods are forwarded to the inner platform.
/// Only compiled for `board-cyd` because `sd_config` (and `tinykv`) are
/// board-cyd only.

use std::cell::RefCell;
use std::vec::Vec;

use common::types::{ChipId, Event, FrameBuffer};
use common::{FileSystem, Platform, PlatformError};

pub struct SdPlatform<'a> {
    inner: &'a dyn Platform,
    /// Interior-mutable FS handle — nvs_* methods need `&mut` on the FS but
    /// Platform trait methods take `&self`.
    fs: RefCell<Box<dyn FileSystem + 'a>>,
}

impl<'a> SdPlatform<'a> {
    pub fn new(inner: &'a dyn Platform, fs: Box<dyn FileSystem + 'a>) -> Self {
        Self {
            inner,
            fs: RefCell::new(fs),
        }
    }
}

impl<'a> Platform for SdPlatform<'a> {
    // ── NVS — redirected to SD tinykv stores ─────────────────────────────────

    fn nvs_read(&self, ns: &str, key: &str) -> Result<Vec<u8>, PlatformError> {
        crate::sd_config::nvs_read(&mut **self.fs.borrow_mut(), ns, key)
    }

    fn nvs_write(&self, ns: &str, key: &str, val: &[u8]) -> Result<(), PlatformError> {
        crate::sd_config::nvs_write(&mut **self.fs.borrow_mut(), ns, key, val)
    }

    fn nvs_delete(&self, ns: &str, key: &str) -> Result<(), PlatformError> {
        crate::sd_config::nvs_erase(&mut **self.fs.borrow_mut(), ns, key)
    }

    // ── SD block access — forward to inner ───────────────────────────────────

    fn sd_read_sectors(&self, start: u32, buf: &mut [u8]) -> Result<(), PlatformError> {
        self.inner.sd_read_sectors(start, buf)
    }

    fn sd_write_sectors(&self, start: u32, buf: &[u8]) -> Result<(), PlatformError> {
        self.inner.sd_write_sectors(start, buf)
    }

    fn sd_sector_count(&self) -> u32 {
        self.inner.sd_sector_count()
    }

    // ── Display ──────────────────────────────────────────────────────────────

    fn display_flush(&self, buf: &FrameBuffer) -> Result<(), PlatformError> {
        self.inner.display_flush(buf)
    }

    fn display_clear(&self) -> Result<(), PlatformError> {
        self.inner.display_clear()
    }

    fn display_width(&self) -> u16 {
        self.inner.display_width()
    }

    fn display_height(&self) -> u16 {
        self.inner.display_height()
    }

    // ── Input ────────────────────────────────────────────────────────────────

    fn poll_event(&self) -> Option<Event> {
        self.inner.poll_event()
    }

    fn uart_poll_byte(&self) -> Option<u8> {
        self.inner.uart_poll_byte()
    }

    fn poll_touch_xy(&self) -> Option<(u16, u16)> {
        self.inner.poll_touch_xy()
    }

    // ── Peripherals ──────────────────────────────────────────────────────────

    fn led_rgb(&self, r: u8, g: u8, b: u8) -> Result<(), PlatformError> {
        self.inner.led_rgb(r, g, b)
    }

    fn battery_percent(&self) -> u8 {
        self.inner.battery_percent()
    }

    fn chip_id(&self) -> ChipId {
        self.inner.chip_id()
    }

    fn reboot(&self) -> ! {
        self.inner.reboot()
    }

    fn sleep_ms(&self, ms: u32) {
        self.inner.sleep_ms(ms)
    }

    fn flashpoint_version(&self) -> (u32, u32) {
        self.inner.flashpoint_version()
    }

    fn wasm_arena_limit(&self) -> usize {
        self.inner.wasm_arena_limit()
    }

    fn lua_heap_limit(&self) -> usize {
        self.inner.lua_heap_limit()
    }

    fn features(&self) -> u64 {
        self.inner.features()
    }
}
