use flashpoint_common::{ChipId, Event};

// ─── FrameBuffer ─────────────────────────────────────────────────────────────

/// A single scanline of pixel data for display output.
///
/// The CYD has no PSRAM — a full 320×240 RGB565 framebuffer (150 KB) won't
/// fit in SRAM. Instead, the boot-rom renders one row at a time and calls
/// display_flush for each. The HAL impl sets the ILI9341 window to that row
/// and DMA-transfers the 640-byte buffer.
pub struct FrameBuffer<'a> {
    /// Which scanline (0 = top row).
    pub y: u16,
    /// RGB565 pixel data: width × 2 bytes.
    pub data: &'a [u8],
}

// ─── PlatformError ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformError {
    SdReadError,
    SdWriteError,
    NvsError,
    DisplayError,
    NotSupported,
}

// ─── Platform trait ──────────────────────────────────────────────────────────

/// The full hardware abstraction contract.
///
/// flash-rom implements this trait for each supported board.
/// boot-rom calls only these methods — zero hardware code in boot-rom.
pub trait Platform {
    // Storage — SD card raw sector I/O
    fn sd_read_sectors(&self, start: u32, buf: &mut [u8])  -> Result<(), PlatformError>;
    fn sd_write_sectors(&self, start: u32, buf: &[u8])     -> Result<(), PlatformError>;
    fn sd_sector_count(&self) -> u32;

    // Storage — KernelFS / NVS
    fn nvs_read(&self, ns: &str, key: &str)                -> Result<alloc::vec::Vec<u8>, PlatformError>;
    fn nvs_write(&self, ns: &str, key: &str, val: &[u8])   -> Result<(), PlatformError>;
    fn nvs_delete(&self, ns: &str, key: &str)              -> Result<(), PlatformError>;

    // Display — scanline-at-a-time for SRAM-constrained devices
    fn display_flush(&self, buf: &FrameBuffer)             -> Result<(), PlatformError>;
    fn display_clear(&self)                                -> Result<(), PlatformError>;
    fn display_width(&self)  -> u16;
    fn display_height(&self) -> u16;

    // Input
    fn poll_event(&self) -> Option<Event>;

    // System
    fn battery_percent(&self) -> u8;
    fn chip_id(&self)         -> ChipId;
    fn reboot(&self)          -> !;
    fn sleep_ms(&self, ms: u32);
}

extern crate alloc;
