#![cfg_attr(not(feature = "std"), no_std)]

use crc::{Crc, CRC_32_ISO_HDLC};

#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

// ─── Header constants ────────────────────────────────────────────────────────

pub const MAGIC: [u8; 4] = *b"FLPT";
pub const HEADER_END_MAGIC: [u8; 4] = *b"FLPE";
pub const HEADER_V1_SIZE: usize = 64;

// Byte offsets within the v2 header block (64 bytes total)
pub const OFF_MAGIC: usize = 0x00; // 4 bytes  "FLPT"
pub const OFF_PLATFORM: usize = 0x04; // 1 byte   primary platform
pub const OFF_ROM_VERSION: usize = 0x05; // 3 bytes  [major, minor, patch]
pub const OFF_BUILT_AGAINST: usize = 0x08; // 4 bytes  Flashpoint API version (LE u32)
pub const OFF_FLAGS: usize = 0x0C; // 2 bytes
pub const OFF_REQUIRED_FEATURES: usize = 0x0E; // 8 bytes  hardware bitmask (LE u64)
pub const OFF_PAYLOAD_LEN: usize = 0x16; // 4 bytes  (LE u32)
pub const OFF_CRC32: usize = 0x1A; // 4 bytes  CRC32 of payload (LE u32)
pub const OFF_PAYLOAD_TYPE: usize = 0x1E; // 1 byte   PayloadType
pub const OFF_ROM_ID: usize = 0x1F; // 24 bytes null-terminated ASCII namespace
pub const OFF_COMPAT_PLATFORMS: usize = 0x37; // 3 bytes  additional supported platforms
pub const OFF_HEADER_SIZE: usize = 0x3A; // 2 bytes  (LE u16), always 64
pub const OFF_HEADER_END: usize = 0x3C; // 4 bytes  "FLPE"

pub const ROM_ID_LEN: usize = 24; // includes null terminator; max 23 usable chars

// ─── Payload type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PayloadType {
    /// Native binary — executed via XIP from flash (must be copied to flash first)
    Native = 0x00,
    /// WASM module — interpreted by wasm3 runtime (can be loaded from SD directly)
    Wasm32 = 0x01,
    /// Compiled LuaC bytecode — interpreted by Lua 5.4 (SD direct, no NVS access)
    Luac54 = 0x02,
}

impl PayloadType {
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(Self::Native),
            0x01 => Some(Self::Wasm32),
            0x02 => Some(Self::Luac54),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wasm32 => "wasm32",
            Self::Luac54 => "luac54",
        }
    }
}

// ─── Flashpoint API versioning ───────────────────────────────────────────────

pub const fn version_pack(major: u8, minor: u8, patch: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (patch as u32)
}

pub fn version_unpack(v: u32) -> (u8, u8, u8) {
    ((v >> 16) as u8, ((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8)
}

pub const FLASHPOINT_CURRENT: u32 = version_pack(0, 2, 0); // bumped for v2 header
pub const FLASHPOINT_LAST_BREAKING: u32 = version_pack(0, 2, 0);

// ─── Platform IDs ────────────────────────────────────────────────────────────

pub const PLATFORM_ESP32: u8 = 0x01;
pub const PLATFORM_ESP32S3: u8 = 0x02;
pub const PLATFORM_RP2040: u8 = 0x03;
pub const PLATFORM_ANY: u8 = 0xFF; // wildcard: ROM runs on any platform

// ─── Feature flags (byte-grouped u64) ───────────────────────────────────────

// Byte 0 — Connectivity
pub const FEAT_WIFI: u64 = 1 << 0;
pub const FEAT_BLE: u64 = 1 << 1;
pub const FEAT_USB_OTG: u64 = 1 << 2;

// Byte 1 — Display
pub const FEAT_DISP_TFT: u64 = 1 << 8;
pub const FEAT_DISP_EINK: u64 = 1 << 9;

// Byte 2 — Input
pub const FEAT_INPUT_TOUCH: u64 = 1 << 16;
pub const FEAT_INPUT_BUTTONS: u64 = 1 << 17;

// Byte 3 — Memory / Power
pub const FEAT_PSRAM: u64 = 1 << 24;
pub const FEAT_BATTERY: u64 = 1 << 25;

pub fn parse_features(s: &str) -> Result<u64, &str> {
    let mut bits = 0u64;
    for part in s.split(',') {
        bits |= match part.trim() {
            "wifi" => FEAT_WIFI,
            "ble" => FEAT_BLE,
            "usb_otg" => FEAT_USB_OTG,
            "disp_tft" => FEAT_DISP_TFT,
            "disp_eink" => FEAT_DISP_EINK,
            "input_touch" => FEAT_INPUT_TOUCH,
            "input_buttons" => FEAT_INPUT_BUTTONS,
            "psram" => FEAT_PSRAM,
            "battery" => FEAT_BATTERY,
            other => return Err(other),
        };
    }
    Ok(bits)
}

#[cfg(feature = "std")]
pub fn features_to_names(bits: u64) -> std::vec::Vec<&'static str> {
    let mut names = std::vec::Vec::new();
    if bits & FEAT_WIFI != 0 {
        names.push("wifi");
    }
    if bits & FEAT_BLE != 0 {
        names.push("ble");
    }
    if bits & FEAT_USB_OTG != 0 {
        names.push("usb_otg");
    }
    if bits & FEAT_DISP_TFT != 0 {
        names.push("disp_tft");
    }
    if bits & FEAT_DISP_EINK != 0 {
        names.push("disp_eink");
    }
    if bits & FEAT_INPUT_TOUCH != 0 {
        names.push("input_touch");
    }
    if bits & FEAT_INPUT_BUTTONS != 0 {
        names.push("input_buttons");
    }
    if bits & FEAT_PSRAM != 0 {
        names.push("psram");
    }
    if bits & FEAT_BATTERY != 0 {
        names.push("battery");
    }
    names
}

// ─── ChipId ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipId {
    Esp32,
    Esp32S3,
    Rp2040,
}

impl ChipId {
    pub fn platform_byte(self) -> u8 {
        match self {
            ChipId::Esp32 => PLATFORM_ESP32,
            ChipId::Esp32S3 => PLATFORM_ESP32S3,
            ChipId::Rp2040 => PLATFORM_RP2040,
        }
    }
}

// ─── Event ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    BtnUp,
    BtnDown,
    BtnLeft,
    BtnRight,
    BtnSelect,
    BtnBack,
    BatteryLow,
    HibernateWarning,
}

// ─── Platform handoff ────────────────────────────────────────────────────────

/// Fixed DRAM address where Stage 1 writes the Platform fat-pointer before
/// jumping to a native boot-rom. Both crates must agree on this value.
///
/// UNRESOLVED (Plan 06): 0x3FFB_0000 is the start of ESP32 SRAM2 and falls
/// within FreeRTOS static allocations (TCBs, queues). Writing here crashes
/// an xQueueSemaphoreTake assertion at boot. A safe address must be found
/// above the FreeRTOS heap (starts at ~0x3FFB_30D0) before enabling the
/// real-hardware jump path. QEMU path intentionally skips this write.
/// WASM/Lua payloads do not use this mechanism — the Platform ref is passed
/// directly into the runtime via host API callbacks.
pub const PLATFORM_PTR_ADDR: usize = 0x3FFB_0000;

// ─── CRC32 ───────────────────────────────────────────────────────────────────

const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub fn crc32(data: &[u8]) -> u32 {
    CRC.checksum(data)
}

// ─── Shared ESP-IDF UART helper ──────────────────────────────────────────────

/// Non-blocking read of one byte from the serial console (stdin / UART0).
///
/// Uses POSIX `read()` on fd 0 with `O_NONBLOCK` through ESP-IDF's VFS layer.
/// This works on every ESP-IDF platform (ESP32, ESP32-S3, QEMU, etc.) without
/// needing `uart_driver_install` — the VFS console backend set up by `binstart`
/// handles the routing to UART0 hardware.
///
/// On non-ESP-IDF targets (host tests, xtask) this always returns `None`.
#[cfg(target_os = "espidf")]
pub fn esp_idf_uart_poll_byte() -> Option<u8> {
    extern "C" {
        fn read(fd: i32, buf: *mut core::ffi::c_void, count: usize) -> isize;
        fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
    }
    // newlib / ESP-IDF constants
    const F_GETFL: i32 = 3;
    const F_SETFL: i32 = 4;
    const O_NONBLOCK: i32 = 0x4000;

    unsafe {
        let flags = fcntl(0, F_GETFL, 0);
        fcntl(0, F_SETFL, flags | O_NONBLOCK);
        let mut byte: u8 = 0;
        let n = read(0, &mut byte as *mut u8 as *mut core::ffi::c_void, 1);
        fcntl(0, F_SETFL, flags); // restore original mode
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

// ─── Header validation ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    TooShort,
    BadMagic,
    WrongPlatform,
    ApiIncompatible,
    BadTerminator,
    UnsupportedHeaderVersion,
    MissingFeatures,
    BadPayloadLen,
    BadChecksum,
    UnknownPayloadType,
}

/// Validate a parsed header byte slice.
///
/// Platform matching: accepts if `our_platform` matches the primary platform
/// byte, any compat_platforms[] entry, or any entry is PLATFORM_ANY (0xFF).
///
/// Returns `Ok(payload_start_offset)` on success.
/// Checksum is verified separately by `verify_crc32()` once the full payload
/// is available.
pub fn validate_header(
    data: &[u8],
    device_features: u64,
    our_platform: u8,
    flashpoint_current: u32,
    flashpoint_last_breaking: u32,
) -> Result<usize, HeaderError> {
    if data.len() < HEADER_V1_SIZE {
        return Err(HeaderError::TooShort);
    }
    if data[OFF_MAGIC..OFF_MAGIC + 4] != MAGIC {
        return Err(HeaderError::BadMagic);
    }

    // Platform check: primary byte or any compat entry, with 0xFF wildcard
    let primary = data[OFF_PLATFORM];
    let compat = &data[OFF_COMPAT_PLATFORMS..OFF_COMPAT_PLATFORMS + 3];
    let platform_ok = primary == PLATFORM_ANY
        || primary == our_platform
        || compat
            .iter()
            .any(|&b| b != 0x00 && (b == our_platform || b == PLATFORM_ANY));
    if !platform_ok {
        return Err(HeaderError::WrongPlatform);
    }

    let built_against = u32::from_le_bytes(
        data[OFF_BUILT_AGAINST..OFF_BUILT_AGAINST + 4]
            .try_into()
            .unwrap(),
    );
    if built_against < flashpoint_last_breaking || built_against > flashpoint_current {
        return Err(HeaderError::ApiIncompatible);
    }

    let hdr_size = u16::from_le_bytes([data[OFF_HEADER_SIZE], data[OFF_HEADER_SIZE + 1]]) as usize;
    if hdr_size < HEADER_V1_SIZE {
        return Err(HeaderError::BadTerminator);
    }
    if hdr_size > HEADER_V1_SIZE {
        return Err(HeaderError::UnsupportedHeaderVersion);
    }
    if data.len() < hdr_size || data[OFF_HEADER_END..OFF_HEADER_END + 4] != HEADER_END_MAGIC {
        return Err(HeaderError::BadTerminator);
    }

    let required = u64::from_le_bytes(
        data[OFF_REQUIRED_FEATURES..OFF_REQUIRED_FEATURES + 8]
            .try_into()
            .unwrap(),
    );
    if device_features & required != required {
        return Err(HeaderError::MissingFeatures);
    }

    let payload_len = u32::from_le_bytes(
        data[OFF_PAYLOAD_LEN..OFF_PAYLOAD_LEN + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    if payload_len == 0 {
        return Err(HeaderError::BadPayloadLen);
    }

    if PayloadType::from_u8(data[OFF_PAYLOAD_TYPE]).is_none() {
        return Err(HeaderError::UnknownPayloadType);
    }

    Ok(hdr_size)
}

/// Verify that `payload` matches the CRC32 stored in `header`.
/// Must be called after `validate_header()` succeeds.
pub fn verify_crc32(header: &[u8], payload: &[u8]) -> Result<(), HeaderError> {
    if header.len() < HEADER_V1_SIZE {
        return Err(HeaderError::TooShort);
    }
    let expected = u32::from_le_bytes(header[OFF_CRC32..OFF_CRC32 + 4].try_into().unwrap());
    let computed = crc32(payload);
    if computed != expected {
        return Err(HeaderError::BadChecksum);
    }
    Ok(())
}

/// Build a v2 header block (exactly HEADER_V1_SIZE = 64 bytes).
pub fn build_header(
    platform: u8,
    rom_version: [u8; 3],
    built_against: u32,
    flags: u16,
    required_features: u64,
    payload_len: u32,
    payload_type: PayloadType,
    rom_id: &str,
    compat_platforms: [u8; 3],
    checksum: u32,
) -> [u8; HEADER_V1_SIZE] {
    let mut h = [0u8; HEADER_V1_SIZE];
    h[OFF_MAGIC..OFF_MAGIC + 4].copy_from_slice(&MAGIC);
    h[OFF_PLATFORM] = platform;
    h[OFF_ROM_VERSION..OFF_ROM_VERSION + 3].copy_from_slice(&rom_version);
    h[OFF_BUILT_AGAINST..OFF_BUILT_AGAINST + 4].copy_from_slice(&built_against.to_le_bytes());
    h[OFF_FLAGS..OFF_FLAGS + 2].copy_from_slice(&flags.to_le_bytes());
    h[OFF_REQUIRED_FEATURES..OFF_REQUIRED_FEATURES + 8]
        .copy_from_slice(&required_features.to_le_bytes());
    h[OFF_PAYLOAD_LEN..OFF_PAYLOAD_LEN + 4].copy_from_slice(&payload_len.to_le_bytes());
    h[OFF_CRC32..OFF_CRC32 + 4].copy_from_slice(&checksum.to_le_bytes());
    h[OFF_PAYLOAD_TYPE] = payload_type as u8;

    // ROM ID: null-terminated, truncated to ROM_ID_LEN bytes
    let id_bytes = rom_id.as_bytes();
    let copy_len = id_bytes.len().min(ROM_ID_LEN - 1);
    h[OFF_ROM_ID..OFF_ROM_ID + copy_len].copy_from_slice(&id_bytes[..copy_len]);
    // remaining bytes already zero (null terminator + padding)

    h[OFF_COMPAT_PLATFORMS..OFF_COMPAT_PLATFORMS + 3].copy_from_slice(&compat_platforms);
    h[OFF_HEADER_SIZE..OFF_HEADER_SIZE + 2].copy_from_slice(&(HEADER_V1_SIZE as u16).to_le_bytes());
    h[OFF_HEADER_END..OFF_HEADER_END + 4].copy_from_slice(&HEADER_END_MAGIC);
    h
}

// ─── Platform HAL types ──────────────────────────────────────────────────────

pub struct FrameBuffer<'a> {
    pub y: u16,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformError {
    SdReadError,
    SdWriteError,
    NvsError,
    DisplayError,
    NotSupported,
}

/// Hardware abstraction contract. firmware implements this per board.
/// kernel / WASM host API calls only these methods — zero hardware code elsewhere.
///
/// Every method has a default implementation so a new HAL crate only needs to
/// override the methods its hardware actually supports.  Unimplemented methods
/// log a warning and return a safe fallback — the boot-rom stays alive even if
/// some subsystem is not yet wired up.
pub trait Platform {
    // ── Storage ──────────────────────────────────────────────────────────────
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

    // ── Display ───────────────────────────────────────────────────────────────
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

    // ── Input ─────────────────────────────────────────────────────────────────
    fn poll_event(&self) -> Option<Event> {
        None
    }

    /// Non-blocking read of one byte from the serial/UART console.
    /// Used by recovery mode for serial interaction alongside display/touch.
    ///
    /// On ESP-IDF targets the default reads from UART0 via `uart_read_bytes`.
    /// HALs only need to override this if their UART setup differs.
    /// On non-ESP-IDF (host tests, xtask) the default returns None.
    fn uart_poll_byte(&self) -> Option<u8> {
        esp_idf_uart_poll_byte()
    }

    /// Non-blocking raw touch sample in device ADC space (0–4095, 0–4095).
    /// Returns `Some((raw_x, raw_y))` while the screen is being touched.
    /// Only meaningful on `FEAT_DISP_TFT | FEAT_INPUT_TOUCH` devices.
    /// The coordinate axes match the XPT2046 ADC channels; no screen-space
    /// calibration is applied — use the values for position deltas and debug
    /// displays rather than pixel-perfect mapping.
    fn poll_touch_xy(&self) -> Option<(u16, u16)> {
        None
    }

    // ── LEDs ──────────────────────────────────────────────────────────────────
    /// Drive the onboard RGB LED. Values are 0=off, 255=full brightness.
    /// Default returns NotSupported (device has no RGB LED).
    fn led_rgb(&self, _r: u8, _g: u8, _b: u8) -> Result<(), PlatformError> {
        log::warn!("led_rgb not supported on this device");
        Err(PlatformError::NotSupported)
    }

    // ── System ────────────────────────────────────────────────────────────────
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
        (FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING)
    }
    /// Max bytes of WASM linear memory this device can provide (0 = no WASM support)
    fn wasm_arena_limit(&self) -> usize {
        0
    }
    /// Max bytes of Lua heap this device can provide (0 = no Lua support)
    fn lua_heap_limit(&self) -> usize {
        0
    }

    // ── Capability reporting ──────────────────────────────────────────────────
    /// Bitmask of FEAT_* constants describing hardware capabilities.
    /// Used by the recovery menu and ROM validation. Default: no features.
    fn features(&self) -> u64 {
        0
    }
}

// ─── Hardware-agnostic kernel entry ─────────────────────────────────────────

// TEMPORARY: embedded orientation test image (128×128 RGB565 LE, 32768 bytes)
#[cfg(feature = "test-image")]
const TEST_IMAGE: &[u8] = include_bytes!(env!("FLASHPOINT_TEST_IMAGE"));
#[cfg(feature = "test-image")]
const TEST_IMAGE_W: u16 = 128;
#[cfg(feature = "test-image")]
const TEST_IMAGE_H: u16 = 128;

#[allow(unreachable_code)]
pub fn boot_main(platform: &dyn Platform) -> ! {
    log::info!("[boot_main] starting");
    let w = platform.display_width();
    let h = platform.display_height();
    log::info!("[boot_main] display {}x{}", w, h);

    // TEMPORARY: render test image centered on screen to verify orientation.
    // The image is rendered exactly as stored — no rotation. If USB port is
    // "up" physically and the heart appears upright, orientation is correct.
    #[cfg(feature = "test-image")]
    {
        log::info!(
            "[boot_main] rendering orientation test image ({}x{})",
            TEST_IMAGE_W,
            TEST_IMAGE_H
        );
        display_fill(platform, 0x0000); // black background
        let x_off = (w.saturating_sub(TEST_IMAGE_W)) / 2;
        let y_off = (h.saturating_sub(TEST_IMAGE_H)) / 2;
        let row_bytes = TEST_IMAGE_W as usize * 2;
        let mut row_buf = [0u8; 640]; // max 320px wide * 2
        for img_y in 0..TEST_IMAGE_H {
            let screen_y = y_off + img_y;
            if screen_y >= h {
                break;
            }
            // Fill row with black
            for i in (0..w as usize * 2).step_by(2) {
                row_buf[i] = 0;
                row_buf[i + 1] = 0;
            }
            // Copy image row into the correct x offset
            let src_start = img_y as usize * row_bytes;
            let src_end = src_start + row_bytes;
            let dst_start = x_off as usize * 2;
            let dst_end = dst_start + row_bytes;
            if src_end <= TEST_IMAGE.len() && dst_end <= row_buf.len() {
                row_buf[dst_start..dst_end].copy_from_slice(&TEST_IMAGE[src_start..src_end]);
            }
            platform
                .display_flush(&FrameBuffer {
                    y: screen_y,
                    data: &row_buf[..w as usize * 2],
                })
                .ok();
        }
        // Label the edges for orientation
        display_text(
            platform,
            x_off,
            y_off.saturating_sub(10),
            "TOP (USB?)",
            0xFFFF,
            0x0000,
        );
        let bottom_y = y_off + TEST_IMAGE_H + 2;
        if bottom_y + 8 <= h {
            display_text(platform, x_off, bottom_y, "BOTTOM", 0xFFFF, 0x0000);
        }
        log::info!("[boot_main] test image rendered — looping forever");
        loop {
            platform.sleep_ms(100);
        }
    }

    display_fill(platform, 0x000F); // dark navy background

    let title = "FLASHPOINT";
    let tx = text_x_center(w, title) as u16;
    display_text(platform, tx, h / 3, title, 0xFFFF, 0x000F);

    let sub = "NO ROM FOUND";
    let sx = text_x_center(w, sub) as u16;
    display_text(platform, sx, h / 3 + 16, sub, 0xFD20, 0x000F);

    let hint = "HOLD BOOT TO RECOVER";
    let hx = text_x_center(w, hint) as u16;
    display_text(platform, hx, h * 3 / 4, hint, 0x07E0, 0x000F);

    log::info!("[boot_main] render complete — entering event loop");
    loop {
        if let Some(Event::BtnSelect) = platform.poll_event() {
            platform.reboot();
        }
        platform.sleep_ms(50);
    }
}

// ── Bitmap font (8×8, public-domain VGA-style glyphs) ────────────────────────

/// Returns the 8 row bytes for an ASCII character.
/// Each byte is one row of pixels, MSB = leftmost pixel.
/// Input is case-insensitive; lowercase is treated as uppercase.
fn font_glyph(c: u8) -> [u8; 8] {
    match c.to_ascii_uppercase() {
        b' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        b'-' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        b':' => [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
        b'/' => [0x00, 0x02, 0x04, 0x08, 0x10, 0x20, 0x00, 0x00],
        b'0' => [0x3C, 0x42, 0x46, 0x4A, 0x52, 0x62, 0x3C, 0x00],
        b'1' => [0x08, 0x18, 0x08, 0x08, 0x08, 0x08, 0x1C, 0x00],
        b'2' => [0x3C, 0x42, 0x02, 0x0C, 0x30, 0x40, 0x7E, 0x00],
        b'3' => [0x3C, 0x42, 0x02, 0x1C, 0x02, 0x42, 0x3C, 0x00],
        b'4' => [0x04, 0x0C, 0x14, 0x24, 0x7E, 0x04, 0x04, 0x00],
        b'5' => [0x7E, 0x40, 0x7C, 0x02, 0x02, 0x42, 0x3C, 0x00],
        b'6' => [0x3C, 0x40, 0x7C, 0x42, 0x42, 0x42, 0x3C, 0x00],
        b'7' => [0x7E, 0x02, 0x04, 0x08, 0x10, 0x20, 0x20, 0x00],
        b'8' => [0x3C, 0x42, 0x42, 0x3C, 0x42, 0x42, 0x3C, 0x00],
        b'9' => [0x3C, 0x42, 0x42, 0x3E, 0x02, 0x42, 0x3C, 0x00],
        b'A' => [0x18, 0x24, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x00],
        b'B' => [0x7C, 0x42, 0x42, 0x7C, 0x42, 0x42, 0x7C, 0x00],
        b'C' => [0x3C, 0x42, 0x40, 0x40, 0x40, 0x42, 0x3C, 0x00],
        b'D' => [0x78, 0x44, 0x42, 0x42, 0x42, 0x44, 0x78, 0x00],
        b'E' => [0x7E, 0x40, 0x40, 0x7C, 0x40, 0x40, 0x7E, 0x00],
        b'F' => [0x7E, 0x40, 0x40, 0x7C, 0x40, 0x40, 0x40, 0x00],
        b'G' => [0x3C, 0x42, 0x40, 0x4E, 0x42, 0x42, 0x3C, 0x00],
        b'H' => [0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x00],
        b'I' => [0x3E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00],
        b'J' => [0x1E, 0x04, 0x04, 0x04, 0x44, 0x44, 0x3C, 0x00],
        b'K' => [0x42, 0x44, 0x48, 0x70, 0x48, 0x44, 0x42, 0x00],
        b'L' => [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00],
        b'M' => [0x42, 0x66, 0x5A, 0x42, 0x42, 0x42, 0x42, 0x00],
        b'N' => [0x42, 0x62, 0x52, 0x4A, 0x46, 0x42, 0x42, 0x00],
        b'O' => [0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00],
        b'P' => [0x7C, 0x42, 0x42, 0x7C, 0x40, 0x40, 0x40, 0x00],
        b'Q' => [0x3C, 0x42, 0x42, 0x42, 0x4A, 0x44, 0x3A, 0x00],
        b'R' => [0x7C, 0x42, 0x42, 0x7C, 0x48, 0x44, 0x42, 0x00],
        b'S' => [0x3C, 0x42, 0x40, 0x3C, 0x02, 0x42, 0x3C, 0x00],
        b'T' => [0x7E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00],
        b'U' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00],
        b'V' => [0x42, 0x42, 0x42, 0x42, 0x24, 0x24, 0x18, 0x00],
        b'W' => [0x42, 0x42, 0x42, 0x42, 0x5A, 0x66, 0x42, 0x00],
        b'X' => [0x42, 0x42, 0x24, 0x18, 0x24, 0x42, 0x42, 0x00],
        b'Y' => [0x42, 0x42, 0x24, 0x18, 0x08, 0x08, 0x08, 0x00],
        b'Z' => [0x7E, 0x02, 0x04, 0x08, 0x10, 0x20, 0x7E, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

/// Write one horizontal scan-line of glyphs into a pixel row buffer (RGB565 LE).
/// `char_row` selects which of the 8 glyph rows to render (0 = top).
/// The caller must pre-fill the row with the background colour before calling this;
/// only pixels covered by set glyph bits are written here.
pub fn draw_text_row(row: &mut [u8], x_start: usize, text: &str, char_row: u8, fg: u16, bg: u16) {
    let fg_b = fg.to_le_bytes();
    let bg_b = bg.to_le_bytes();
    let row_px = row.len() / 2; // total pixels in this row
    for (ci, c) in text.bytes().enumerate() {
        let glyph_row = font_glyph(c)[char_row as usize];
        for bit in 0..8usize {
            // Display scans right-to-left: mirror x so text reads correctly.
            let logical_x = x_start + ci * 8 + bit;
            let px = (row_px.saturating_sub(1 + logical_x)) * 2;
            if px + 1 >= row.len() {
                continue;
            }
            let color = if (glyph_row >> (7 - bit)) & 1 != 0 {
                fg_b
            } else {
                bg_b
            };
            row[px] = color[0];
            row[px + 1] = color[1];
        }
    }
}

/// Returns the x pixel offset to horizontally center `text` in a row of `w` pixels.
pub fn text_x_center(w: u16, text: &str) -> usize {
    let tw = text.len() * 8;
    if tw >= w as usize {
        0
    } else {
        (w as usize - tw) / 2
    }
}

/// Fill the entire display with a single RGB565 colour.
pub fn display_fill(platform: &dyn Platform, color: u16) {
    let w = platform.display_width();
    let h = platform.display_height();
    let mut row = [0u8; 640];
    let b = color.to_le_bytes();
    for i in (0..w as usize * 2).step_by(2) {
        row[i] = b[0];
        row[i + 1] = b[1];
    }
    for y in 0..h {
        platform
            .display_flush(&FrameBuffer {
                y,
                data: &row[..w as usize * 2],
            })
            .ok();
    }
}

/// Render a single line of 8px-tall text at pixel position (x, y).
/// Writes exactly 8 full-width rows to the display starting at row y.
pub fn display_text(platform: &dyn Platform, x: u16, y: u16, text: &str, fg: u16, bg: u16) {
    let w = platform.display_width();
    let mut row = [0u8; 640];
    let bg_b = bg.to_le_bytes();
    for row_i in 0u16..8 {
        for i in (0..w as usize * 2).step_by(2) {
            row[i] = bg_b[0];
            row[i + 1] = bg_b[1];
        }
        draw_text_row(
            &mut row[..w as usize * 2],
            x as usize,
            text,
            row_i as u8,
            fg,
            bg,
        );
        platform
            .display_flush(&FrameBuffer {
                y: y + row_i,
                data: &row[..w as usize * 2],
            })
            .ok();
    }
}

// ─── Recovery menu ───────────────────────────────────────────────────────────

/// Recovery menu items.  The list is fixed; capability-gated items are shown
/// or hidden at runtime based on `platform.features()`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RecoveryItem {
    DisplayTest,
    TouchCalib,
    LedTest,
    WifiAp,   // only shown when FEAT_WIFI
    UsbMount, // only shown when FEAT_USB_OTG
    Reboot,
}

impl RecoveryItem {
    /// RGB565 colour for the band when this item is the active selection.
    fn color_active(self) -> u16 {
        match self {
            RecoveryItem::DisplayTest => 0xF81F, // magenta
            RecoveryItem::TouchCalib => 0x07FF,  // cyan
            RecoveryItem::LedTest => 0xFFE0,     // yellow
            RecoveryItem::WifiAp => 0x001F,      // blue
            RecoveryItem::UsbMount => 0x07E0,    // green
            RecoveryItem::Reboot => 0xF800,      // red
        }
    }
    /// Dimmed (inactive) version: shift right 2 bits per channel.
    fn color_inactive(self) -> u16 {
        let c = self.color_active();
        let r = (c >> 11) & 0x1F;
        let g = (c >> 5) & 0x3F;
        let b = c & 0x1F;
        ((r >> 2) << 11) | ((g >> 2) << 5) | (b >> 2)
    }
    fn label(self) -> &'static str {
        match self {
            RecoveryItem::DisplayTest => "DISPLAY TEST",
            RecoveryItem::TouchCalib => "TOUCH CALIBRATION",
            RecoveryItem::LedTest => "LED TEST",
            RecoveryItem::WifiAp => "WIFI AP RECOVERY",
            RecoveryItem::UsbMount => "USB MOUNT SD",
            RecoveryItem::Reboot => "REBOOT",
        }
    }
}

// ─── UART recovery input ─────────────────────────────────────────────────────

/// Map a UART byte to a navigation Event.
/// Supports: w/k=Up, s/j=Down, enter/space=Select, q/ESC=Back.
#[cfg(not(feature = "no-uart-recovery"))]
fn uart_byte_to_event(byte: u8) -> Option<Event> {
    match byte {
        b'w' | b'W' | b'k' | b'K' => Some(Event::BtnUp),
        b's' | b'S' | b'j' | b'J' => Some(Event::BtnDown),
        b'a' | b'A' | b'h' | b'H' => Some(Event::BtnLeft),
        b'd' | b'D' | b'l' | b'L' => Some(Event::BtnRight),
        b'\r' | b'\n' | b' ' => Some(Event::BtnSelect),
        b'q' | b'Q' | 0x1B => Some(Event::BtnBack),
        _ => None,
    }
}

/// Check for a direct numeric selection (keys '1'-'9') from UART.
/// Returns the 0-based item index, or None.
#[cfg(not(feature = "no-uart-recovery"))]
fn uart_byte_to_index(byte: u8, item_count: usize) -> Option<usize> {
    if byte >= b'1' && byte <= b'9' {
        let idx = (byte - b'1') as usize;
        if idx < item_count {
            return Some(idx);
        }
    }
    None
}

/// Unified input: polls hardware events first, then UART.
/// Returns (Option<Event>, Option<raw_uart_byte>).
#[cfg(not(feature = "no-uart-recovery"))]
fn poll_recovery_input(platform: &dyn Platform) -> (Option<Event>, Option<u8>) {
    if let Some(e) = platform.poll_event() {
        return (Some(e), None);
    }
    if let Some(byte) = platform.uart_poll_byte() {
        return (uart_byte_to_event(byte), Some(byte));
    }
    (None, None)
}

/// Simple any-input check: returns true if hardware or UART produced any event.
/// Used by recovery actions that just need "wait for any key/touch".
fn any_recovery_input(platform: &dyn Platform) -> bool {
    if platform.poll_event().is_some() {
        return true;
    }
    #[cfg(not(feature = "no-uart-recovery"))]
    if platform.uart_poll_byte().is_some() {
        return true;
    }
    false
}

/// Log the recovery menu over UART so serial users can see the options.
#[cfg(not(feature = "no-uart-recovery"))]
fn uart_log_menu(items: &[RecoveryItem], selected: usize) {
    log::info!("[recovery] ─── RECOVERY MENU ───");
    for (i, item) in items.iter().enumerate() {
        let marker = if i == selected { ">>" } else { "  " };
        log::info!("[recovery] {} [{}] {}", marker, i + 1, item.label());
    }
    log::info!(
        "[recovery] Navigate: w/s or k/j | Select: Enter/Space | Direct: 1-{}",
        items.len()
    );
}

/// Hardware-agnostic recovery menu.
///
/// - If the platform has a display (`FEAT_DISP_TFT`), renders a colour-band
///   menu and navigates with touch/button events.
/// - Otherwise falls back to the serial console: interactive UART menu.
///
/// UART console access is always active (both display and console paths)
/// unless the `no-uart-recovery` build feature is set.
pub fn recovery_main(platform: &dyn Platform) -> ! {
    log::info!("[recovery] entering recovery mode");

    let has_display = platform.features() & FEAT_DISP_TFT != 0;
    let has_wifi = platform.features() & FEAT_WIFI != 0;
    let has_usb_otg = platform.features() & FEAT_USB_OTG != 0;

    if has_display {
        recovery_display_menu(platform, has_wifi, has_usb_otg)
    } else {
        recovery_console(platform, has_wifi, has_usb_otg)
    }
}

fn build_recovery_items(has_display: bool, has_wifi: bool, has_usb_otg: bool) -> Vec<RecoveryItem> {
    let mut items: Vec<RecoveryItem> = Vec::new();
    if has_display {
        items.push(RecoveryItem::DisplayTest);
        items.push(RecoveryItem::TouchCalib);
    }
    items.push(RecoveryItem::LedTest);
    if has_wifi {
        items.push(RecoveryItem::WifiAp);
    }
    if has_usb_otg {
        items.push(RecoveryItem::UsbMount);
    }
    items.push(RecoveryItem::Reboot);
    items
}

fn recovery_display_menu(platform: &dyn Platform, has_wifi: bool, has_usb_otg: bool) -> ! {
    let items = build_recovery_items(true, has_wifi, has_usb_otg);

    let mut selected: usize = 0;
    let w = platform.display_width();
    let h = platform.display_height();
    let n = items.len() as u16;
    let band = h / n;

    // Draw initial menu + log over UART
    recovery_draw_menu(platform, &items, selected, w, h, band);
    #[cfg(not(feature = "no-uart-recovery"))]
    uart_log_menu(&items, selected);

    loop {
        // Unified input: hardware events + UART (unless no-uart-recovery)
        #[cfg(not(feature = "no-uart-recovery"))]
        {
            let (event, raw_byte) = poll_recovery_input(platform);
            // Direct number key selection
            if let Some(byte) = raw_byte {
                if let Some(idx) = uart_byte_to_index(byte, items.len()) {
                    selected = idx;
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                    platform.sleep_ms(50);
                    continue;
                }
            }
            match event {
                Some(Event::BtnUp) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnDown) => {
                    if selected + 1 < items.len() {
                        selected += 1;
                    }
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnSelect) => {
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    recovery_draw_menu(platform, &items, selected, w, h, band);
                    uart_log_menu(&items, selected);
                }
                _ => {}
            }
        }
        #[cfg(feature = "no-uart-recovery")]
        match platform.poll_event() {
            Some(Event::BtnUp) => {
                if selected > 0 {
                    selected -= 1;
                }
                recovery_draw_menu(platform, &items, selected, w, h, band);
            }
            Some(Event::BtnDown) => {
                if selected + 1 < items.len() {
                    selected += 1;
                }
                recovery_draw_menu(platform, &items, selected, w, h, band);
            }
            Some(Event::BtnSelect) => {
                recovery_run_item(platform, items[selected]);
                recovery_draw_menu(platform, &items, selected, w, h, band);
            }
            _ => {}
        }
        platform.sleep_ms(50);
    }
}

fn recovery_draw_menu(
    platform: &dyn Platform,
    items: &[RecoveryItem],
    selected: usize,
    w: u16,
    h: u16,
    band: u16,
) {
    let mut row = [0u8; 640];
    let n = items.len() as u16;
    for y in 0..h {
        let item_idx = ((y / band) as usize).min(n as usize - 1);
        let active = item_idx == selected;
        let bg = if active {
            items[item_idx].color_active()
        } else {
            items[item_idx].color_inactive()
        };
        // Fill row with band colour.
        let b = bg.to_le_bytes();
        for i in (0..w as usize * 2).step_by(2) {
            row[i] = b[0];
            row[i + 1] = b[1];
        }
        // Overlay text label centred vertically within the band.
        let band_start = item_idx as u16 * band;
        let text_top = band_start + band.saturating_sub(8) / 2;
        if y >= text_top && y < text_top + 8 {
            let label = items[item_idx].label();
            let char_row = (y - text_top) as u8;
            let lx = text_x_center(w, label);
            // Black text on bright (selected) band, white on dimmed (unselected).
            let fg: u16 = if active { 0x0000 } else { 0xFFFF };
            draw_text_row(&mut row[..w as usize * 2], lx, label, char_row, fg, bg);
        }
        platform
            .display_flush(&FrameBuffer {
                y,
                data: &row[..w as usize * 2],
            })
            .ok();
    }
}

/// Render a touch calibration target screen: black background with a cyan crosshair
/// at pixel (tx, ty) and the instruction label centred near the top.
fn recovery_cal_render(platform: &dyn Platform, tx: u16, ty: u16, label: &str) {
    let w = platform.display_width();
    let h = platform.display_height();
    let mut row = [0u8; 640];
    let b_bg = (0x0000u16).to_le_bytes();
    let b_ch = (0x07FFu16).to_le_bytes(); // cyan crosshair
    for y in 0..h {
        for i in (0..w as usize * 2).step_by(2) {
            row[i] = b_bg[0];
            row[i + 1] = b_bg[1];
        }
        if y == ty {
            for i in (0..w as usize * 2).step_by(2) {
                row[i] = b_ch[0];
                row[i + 1] = b_ch[1];
            }
        }
        let px = tx as usize * 2;
        if px + 1 < w as usize * 2 {
            row[px] = b_ch[0];
            row[px + 1] = b_ch[1];
        }
        if y >= 4 && y < 12 {
            let lx = text_x_center(w, label);
            draw_text_row(&mut row[..w as usize * 2], lx, label, (y - 4) as u8, 0xFFFF, 0x0000);
        }
        platform
            .display_flush(&FrameBuffer {
                y,
                data: &row[..w as usize * 2],
            })
            .ok();
    }
}

/// Collect a stable touch sample: waits for 10 consecutive `poll_touch_xy()` readings
/// (50 ms apart) and returns their average. Resets the counter if the finger lifts.
/// Wait until the screen reports no touch for at least two consecutive polls.
/// Call this before `recovery_cal_sample` to flush any residual touch from
/// the previous menu tap or calibration step.
fn wait_for_no_touch(platform: &dyn Platform) {
    let mut clear = 0u32;
    while clear < 2 {
        if platform.poll_touch_xy().is_none() {
            clear += 1;
        } else {
            clear = 0;
        }
        platform.sleep_ms(50);
    }
}

fn recovery_cal_sample(platform: &dyn Platform) -> (u16, u16) {
    // Ensure any previous touch (e.g. menu selection tap) is fully lifted
    // before we start accumulating calibration samples.
    wait_for_no_touch(platform);

    const NEEDED: u32 = 10;
    let mut sum_x = 0u32;
    let mut sum_y = 0u32;
    let mut count = 0u32;
    loop {
        match platform.poll_touch_xy() {
            Some((x, y)) => {
                sum_x += x as u32;
                sum_y += y as u32;
                count += 1;
                log::info!("[cal] sample {}/{}: ({}, {})", count, NEEDED, x, y);
                if count >= NEEDED {
                    return ((sum_x / NEEDED) as u16, (sum_y / NEEDED) as u16);
                }
            }
            None => {
                if count > 0 {
                    log::warn!("[cal] lifted early ({} samples), retrying", count);
                    sum_x = 0;
                    sum_y = 0;
                    count = 0;
                }
            }
        }
        platform.sleep_ms(50);
    }
}

fn recovery_run_item(platform: &dyn Platform, item: RecoveryItem) {
    match item {
        RecoveryItem::DisplayTest => {
            log::info!("[recovery] running display test");
            let w = platform.display_width();
            let h = platform.display_height();
            let mut row = [0u8; 640];
            // Draw RGB stripes: red / green / blue / white / black
            let stripe_h = h / 5;
            let colors: [u16; 5] = [0xF800, 0x07E0, 0x001F, 0xFFFF, 0x0000];
            for y in 0..h {
                let c = colors[((y / stripe_h) as usize).min(4)];
                let b = c.to_le_bytes();
                for i in (0..w as usize * 2).step_by(2) {
                    row[i] = b[0];
                    row[i + 1] = b[1];
                }
                platform
                    .display_flush(&FrameBuffer {
                        y,
                        data: &row[..w as usize * 2],
                    })
                    .ok();
            }
            log::info!("[recovery] display test — any input to exit");
            loop {
                if any_recovery_input(platform) {
                    break;
                }
                platform.sleep_ms(50);
            }
        }
        RecoveryItem::TouchCalib => {
            // Two-point touch calibration wizard (TFT devices only).
            // Guides the user to tap a crosshair at the top-left then bottom-right
            // corners of the screen.  Raw XPT2046 ADC values at each tap are
            // averaged over 10 samples, then stored to NVS so the HAL can apply
            // accurate proportional zone mapping on the next boot.
            //
            // NVS layout — ns: "fp-hal", key: "touch-cal", 8 bytes:
            //   [x_min_lo, x_min_hi, x_max_lo, x_max_hi,
            //    y_min_lo, y_min_hi, y_max_lo, y_max_hi]
            log::info!("[recovery] entering touch calibration wizard");
            let w = platform.display_width();
            let h = platform.display_height();

            // ── Step 1: tap top-left ──────────────────────────────────────────
            log::info!("[cal] step 1/2 — tap the TOP-LEFT crosshair and hold");
            recovery_cal_render(platform, 20, 20, "TAP TOP LEFT");
            let (x1, y1) = recovery_cal_sample(platform);
            log::info!("[cal] top-left averaged raw: ({}, {})", x1, y1);
            display_fill(platform, 0x07E0); // green flash = confirmed
            platform.sleep_ms(300);

            // ── Step 2: tap bottom-right ──────────────────────────────────────
            let br_x = w.saturating_sub(21);
            let br_y = h.saturating_sub(21);
            log::info!("[cal] step 2/2 — tap the BOTTOM-RIGHT crosshair and hold");
            recovery_cal_render(platform, br_x, br_y, "TAP BOTTOM RIGHT");
            let (x2, y2) = recovery_cal_sample(platform);
            log::info!("[cal] bottom-right averaged raw: ({}, {})", x2, y2);
            display_fill(platform, 0x07E0);
            platform.sleep_ms(300);

            // ── Compute calibration bounds ─────────────────────────────────────
            let x_min = x1.min(x2);
            let x_max = x1.max(x2);
            let y_min = y1.min(y2);
            let y_max = y1.max(y2);
            log::info!(
                "[cal] calibration bounds: x {}..{}, y {}..{}",
                x_min, x_max, y_min, y_max
            );

            // ── Encode and write to NVS ────────────────────────────────────────
            let mut cal_bytes = [0u8; 8];
            cal_bytes[0..2].copy_from_slice(&x_min.to_le_bytes());
            cal_bytes[2..4].copy_from_slice(&x_max.to_le_bytes());
            cal_bytes[4..6].copy_from_slice(&y_min.to_le_bytes());
            cal_bytes[6..8].copy_from_slice(&y_max.to_le_bytes());

            display_fill(platform, 0x0000);
            let status = match platform.nvs_write("fp-hal", "touch-cal", &cal_bytes) {
                Ok(()) => {
                    log::info!("[cal] calibration saved to NVS — rebooting to apply");
                    "SAVED"
                }
                Err(e) => {
                    log::error!("[cal] NVS write failed: {:?}", e);
                    "NVS FAILED"
                }
            };
            let sx = text_x_center(w, status) as u16;
            display_text(platform, sx, h / 2, status, 0xFFFF, 0x0000);
            platform.sleep_ms(1500);
            platform.reboot();
        }
        RecoveryItem::LedTest => {
            log::info!("[recovery] running LED test");
            let seq: [(u8, u8, u8); 6] = [
                (255, 0, 0),
                (0, 255, 0),
                (0, 0, 255),
                (255, 255, 0),
                (255, 255, 255),
                (0, 0, 0),
            ];
            for (r, g, b) in seq {
                if platform.led_rgb(r, g, b).is_err() {
                    log::warn!("[recovery] LED not available on this device");
                    break;
                }
                platform.sleep_ms(400);
            }
        }
        RecoveryItem::WifiAp => {
            log::info!("[recovery] WiFi AP recovery — not yet implemented");
            // Future: platform.wifi_start_ap("flashpoint-recovery", "") + HTTP file server
            platform.sleep_ms(1000);
        }
        RecoveryItem::UsbMount => {
            log::info!("[recovery] USB SD mount — not yet implemented");
            // Future: expose SD card as USB mass storage device so the user can
            // transfer ROMs to/from the SD card without removing it physically.
            // Boot-ROMs may implement their own version of this via host API.
            platform.sleep_ms(1000);
        }
        RecoveryItem::Reboot => {
            log::info!("[recovery] rebooting...");
            platform.sleep_ms(500);
            platform.reboot();
        }
    }
}

fn recovery_console(platform: &dyn Platform, has_wifi: bool, has_usb_otg: bool) -> ! {
    log::info!("[recovery] ---- RECOVERY MODE (console) ----");

    #[cfg(not(feature = "no-uart-recovery"))]
    {
        // Interactive UART console: present menu, accept commands
        let items = build_recovery_items(false, has_wifi, has_usb_otg);
        let mut selected: usize = 0;
        uart_log_menu(&items, selected);

        loop {
            let (event, raw_byte) = poll_recovery_input(platform);
            // Direct number key selection
            if let Some(byte) = raw_byte {
                if let Some(idx) = uart_byte_to_index(byte, items.len()) {
                    selected = idx;
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    uart_log_menu(&items, selected);
                    platform.sleep_ms(50);
                    continue;
                }
            }
            match event {
                Some(Event::BtnUp) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnDown) => {
                    if selected + 1 < items.len() {
                        selected += 1;
                    }
                    uart_log_menu(&items, selected);
                }
                Some(Event::BtnSelect) => {
                    log::info!("[recovery] running: {}", items[selected].label());
                    recovery_run_item(platform, items[selected]);
                    uart_log_menu(&items, selected);
                }
                _ => {}
            }
            platform.sleep_ms(50);
        }
    }

    // Fallback: no UART recovery — run tests automatically and reboot
    #[cfg(feature = "no-uart-recovery")]
    {
        let _ = (has_wifi, has_usb_otg);
        log::info!("[recovery] running display test...");
        platform.display_clear().ok();
        platform.sleep_ms(500);

        log::info!("[recovery] running LED test...");
        for (r, g, b) in [
            (255u8, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 255),
            (0u8, 0, 0),
        ] {
            if platform.led_rgb(r, g, b).is_err() {
                log::warn!("[recovery] LED not available");
                break;
            }
            platform.sleep_ms(400);
        }

        log::info!("[recovery] tests complete — rebooting in 3s");
        platform.sleep_ms(3000);
        platform.reboot();
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_crc() -> u32 {
        0xDEAD_BEEF
    }

    fn make_valid_header() -> [u8; HEADER_V1_SIZE] {
        let payload = b"test payload";
        let checksum = crc32(payload);
        build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            0,
            payload.len() as u32,
            PayloadType::Native,
            "com.test",
            [0, 0, 0],
            checksum,
        )
    }

    #[test]
    fn round_trip_header_fields() {
        let h = make_valid_header();
        assert_eq!(&h[OFF_MAGIC..OFF_MAGIC + 4], &MAGIC);
        assert_eq!(h[OFF_PLATFORM], PLATFORM_ESP32);
        assert_eq!(&h[OFF_HEADER_END..OFF_HEADER_END + 4], &HEADER_END_MAGIC);
        assert_eq!(
            u16::from_le_bytes([h[OFF_HEADER_SIZE], h[OFF_HEADER_SIZE + 1]]),
            HEADER_V1_SIZE as u16
        );
        assert_eq!(h[OFF_PAYLOAD_TYPE], PayloadType::Native as u8);
        assert_eq!(&h[OFF_ROM_ID..OFF_ROM_ID + 8], b"com.test");
        assert_eq!(h[OFF_ROM_ID + 8], 0x00); // null terminator
    }

    #[test]
    fn validate_rejects_bad_magic() {
        let mut h = make_valid_header();
        h[0] = 0xFF;
        assert_eq!(
            validate_header(
                &h,
                0,
                PLATFORM_ESP32,
                FLASHPOINT_CURRENT,
                FLASHPOINT_LAST_BREAKING
            ),
            Err(HeaderError::BadMagic)
        );
    }

    #[test]
    fn validate_rejects_wrong_platform() {
        let h = make_valid_header();
        assert_eq!(
            validate_header(
                &h,
                0,
                PLATFORM_ESP32S3,
                FLASHPOINT_CURRENT,
                FLASHPOINT_LAST_BREAKING
            ),
            Err(HeaderError::WrongPlatform)
        );
    }

    #[test]
    fn validate_accepts_compat_platform() {
        // Build ROM targeting ESP32S3 primary + ESP32 as compat
        let payload = b"x";
        let h = build_header(
            PLATFORM_ESP32S3,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            0,
            1,
            PayloadType::Native,
            "",
            [PLATFORM_ESP32, 0, 0],
            crc32(payload),
        );
        assert!(validate_header(
            &h,
            0,
            PLATFORM_ESP32,
            FLASHPOINT_CURRENT,
            FLASHPOINT_LAST_BREAKING
        )
        .is_ok());
    }

    #[test]
    fn validate_accepts_platform_any_wildcard() {
        let payload = b"x";
        let h = build_header(
            PLATFORM_ANY,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            0,
            1,
            PayloadType::Native,
            "",
            [0, 0, 0],
            crc32(payload),
        );
        assert!(validate_header(
            &h,
            0,
            PLATFORM_RP2040,
            FLASHPOINT_CURRENT,
            FLASHPOINT_LAST_BREAKING
        )
        .is_ok());
    }

    #[test]
    fn validate_rejects_api_incompatible_too_old() {
        let future_ver = version_pack(1, 0, 0);
        let h = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            future_ver,
            0,
            0,
            1,
            PayloadType::Native,
            "",
            [0, 0, 0],
            dummy_crc(),
        );
        assert_eq!(
            validate_header(
                &h,
                0,
                PLATFORM_ESP32,
                FLASHPOINT_CURRENT,
                FLASHPOINT_LAST_BREAKING
            ),
            Err(HeaderError::ApiIncompatible)
        );
    }

    #[test]
    fn validate_rejects_missing_features() {
        let h = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            FEAT_PSRAM,
            1,
            PayloadType::Native,
            "",
            [0, 0, 0],
            dummy_crc(),
        );
        assert_eq!(
            validate_header(
                &h,
                0,
                PLATFORM_ESP32,
                FLASHPOINT_CURRENT,
                FLASHPOINT_LAST_BREAKING
            ),
            Err(HeaderError::MissingFeatures)
        );
    }

    #[test]
    fn validate_passes_with_features_met() {
        let h = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            FEAT_PSRAM,
            1,
            PayloadType::Native,
            "",
            [0, 0, 0],
            dummy_crc(),
        );
        assert!(validate_header(
            &h,
            FEAT_PSRAM | FEAT_WIFI,
            PLATFORM_ESP32,
            FLASHPOINT_CURRENT,
            FLASHPOINT_LAST_BREAKING
        )
        .is_ok());
    }

    #[test]
    fn validate_rejects_bad_terminator() {
        let mut h = make_valid_header();
        h[OFF_HEADER_END] = 0x00;
        assert_eq!(
            validate_header(
                &h,
                0,
                PLATFORM_ESP32,
                FLASHPOINT_CURRENT,
                FLASHPOINT_LAST_BREAKING
            ),
            Err(HeaderError::BadTerminator)
        );
    }

    #[test]
    fn verify_crc32_accepts_correct_payload() {
        let payload = b"hello flashpoint";
        let checksum = crc32(payload);
        let h = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            0,
            payload.len() as u32,
            PayloadType::Native,
            "",
            [0, 0, 0],
            checksum,
        );
        assert!(verify_crc32(&h, payload).is_ok());
    }

    #[test]
    fn verify_crc32_rejects_corrupted_payload() {
        let payload = b"hello flashpoint";
        let checksum = crc32(payload);
        let h = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            0,
            payload.len() as u32,
            PayloadType::Native,
            "",
            [0, 0, 0],
            checksum,
        );
        assert_eq!(
            verify_crc32(&h, b"hello flashpointX"),
            Err(HeaderError::BadChecksum)
        );
    }

    #[test]
    fn validate_rejects_unknown_payload_type() {
        let mut h = make_valid_header();
        h[OFF_PAYLOAD_TYPE] = 0xFF;
        assert_eq!(
            validate_header(
                &h,
                0,
                PLATFORM_ESP32,
                FLASHPOINT_CURRENT,
                FLASHPOINT_LAST_BREAKING
            ),
            Err(HeaderError::UnknownPayloadType)
        );
    }

    #[test]
    fn parse_features_round_trip() {
        let bits = parse_features("psram,wifi,disp_tft").unwrap();
        assert_eq!(bits, FEAT_PSRAM | FEAT_WIFI | FEAT_DISP_TFT);
    }

    #[test]
    fn header_size_is_64() {
        assert_eq!(HEADER_V1_SIZE, 64);
        assert_eq!(OFF_HEADER_END, 0x3C);
    }

    #[test]
    fn version_pack_unpack_round_trip() {
        let v = version_pack(1, 2, 3);
        assert_eq!(version_unpack(v), (1, 2, 3));
    }

    #[test]
    fn feature_flags_are_in_correct_bytes() {
        assert!(FEAT_WIFI < (1 << 8));
        assert!(FEAT_DISP_TFT >= (1 << 8) && FEAT_DISP_TFT < (1 << 16));
        assert!(FEAT_INPUT_TOUCH >= (1 << 16) && FEAT_INPUT_TOUCH < (1 << 24));
        assert!(FEAT_PSRAM >= (1 << 24));
    }

    #[test]
    fn rom_id_truncates_to_23_chars() {
        let long_id = "com.example.toolongidentifier.truncated";
        let payload = b"x";
        let h = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            0,
            1,
            PayloadType::Wasm32,
            long_id,
            [0, 0, 0],
            crc32(payload),
        );
        // Byte 23 (index from OFF_ROM_ID) must be null terminator
        assert_eq!(h[OFF_ROM_ID + 23], 0x00);
    }

    #[test]
    fn header_end_magic_is_flpe() {
        let h = make_valid_header();
        assert_eq!(&h[OFF_HEADER_END..OFF_HEADER_END + 4], b"FLPE");
    }
}
