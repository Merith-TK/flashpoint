#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

// ─── Header constants ────────────────────────────────────────────────────────

pub const MAGIC: [u8; 4]        = *b"FLPT";
pub const HEADER_V1_SIZE: usize = 64;
pub const HEADER_END_MAGIC: u8  = 0xFE;

// Byte offsets within the header block
pub const OFF_MAGIC:             usize = 0x00; // 4 bytes
pub const OFF_PLATFORM:          usize = 0x04; // 1 byte
pub const OFF_ROM_VERSION:       usize = 0x05; // 3 bytes
pub const OFF_BUILT_AGAINST:     usize = 0x08; // 4 bytes  — Flashpoint API version ROM targets
pub const OFF_FLAGS:             usize = 0x0C; // 2 bytes
pub const OFF_REQUIRED_FEATURES: usize = 0x0E; // 8 bytes
pub const OFF_PAYLOAD_LEN:       usize = 0x16; // 4 bytes
pub const OFF_CHECKSUM:          usize = 0x1A; // 32 bytes
pub const OFF_HEADER_SIZE:       usize = 0x3A; // 2 bytes
pub const OFF_RESERVED:          usize = 0x3C; // 3 bytes
pub const OFF_HEADER_END:        usize = 0x3F; // 1 byte

// ─── Flashpoint API versioning ───────────────────────────────────────────────

/// Pack major.minor.patch into a u32: `(major << 16) | (minor << 8) | patch`.
/// Each component is 0–255, matching the rom_version [u8; 3] range.
pub const fn version_pack(major: u8, minor: u8, patch: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (patch as u32)
}

/// Unpack a version u32 back to (major, minor, patch).
pub fn version_unpack(v: u32) -> (u8, u8, u8) {
    ((v >> 16) as u8, ((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8)
}

/// The Flashpoint API version this firmware build implements.
pub const FLASHPOINT_CURRENT: u32       = version_pack(0, 1, 0);
/// The oldest Flashpoint API version that is still wire-compatible with the current build.
/// A Boot-ROM built against any version >= FLASHPOINT_LAST_BREAKING can run on this firmware.
pub const FLASHPOINT_LAST_BREAKING: u32 = version_pack(0, 1, 0);

// ─── Platform IDs ────────────────────────────────────────────────────────────

pub const PLATFORM_ESP32:   u8 = 0x01;
pub const PLATFORM_ESP32S3: u8 = 0x02;
pub const PLATFORM_RP2040:  u8 = 0x03;
pub const PLATFORM_MULTI:   u8 = 0xFF; // future: multi-platform rom

// ─── Feature flags (byte-grouped u64) ───────────────────────────────────────
//
// Bits are grouped into logical bytes so hex dumps are readable and each
// category has room to grow without renumbering other groups.

// Byte 0 — Connectivity (bits 0–7)
pub const FEAT_WIFI:          u64 = 1 << 0;
pub const FEAT_BLE:           u64 = 1 << 1;
pub const FEAT_USB_OTG:       u64 = 1 << 2;
// bits 3–7 reserved

// Byte 1 — Display (bits 8–15)
pub const FEAT_DISP_TFT:      u64 = 1 << 8;
pub const FEAT_DISP_EINK:     u64 = 1 << 9;
// bits 10–15 reserved

// Byte 2 — Input (bits 16–23)
pub const FEAT_INPUT_TOUCH:   u64 = 1 << 16;
pub const FEAT_INPUT_BUTTONS: u64 = 1 << 17;
// bits 18–23 reserved

// Byte 3 — Memory / Power (bits 24–31)
pub const FEAT_PSRAM:         u64 = 1 << 24;
pub const FEAT_BATTERY:       u64 = 1 << 25;
// bits 26–31 reserved

// Bytes 4–7 reserved for future feature groups

/// Parse a comma-separated feature string into a bitmask.
/// e.g. "wifi,disp_tft" → FEAT_WIFI | FEAT_DISP_TFT
pub fn parse_features(s: &str) -> Result<u64, &str> {
    let mut bits = 0u64;
    for part in s.split(',') {
        bits |= match part.trim() {
            "wifi"          => FEAT_WIFI,
            "ble"           => FEAT_BLE,
            "usb_otg"       => FEAT_USB_OTG,
            "disp_tft"      => FEAT_DISP_TFT,
            "disp_eink"     => FEAT_DISP_EINK,
            "input_touch"   => FEAT_INPUT_TOUCH,
            "input_buttons" => FEAT_INPUT_BUTTONS,
            "psram"         => FEAT_PSRAM,
            "battery"       => FEAT_BATTERY,
            other           => return Err(other),
        };
    }
    Ok(bits)
}

/// Human-readable list of feature names from a bitmask.
#[cfg(feature = "std")]
pub fn features_to_names(bits: u64) -> std::vec::Vec<&'static str> {
    let mut names = std::vec::Vec::new();
    if bits & FEAT_WIFI          != 0 { names.push("wifi"); }
    if bits & FEAT_BLE           != 0 { names.push("ble"); }
    if bits & FEAT_USB_OTG       != 0 { names.push("usb_otg"); }
    if bits & FEAT_DISP_TFT      != 0 { names.push("disp_tft"); }
    if bits & FEAT_DISP_EINK     != 0 { names.push("disp_eink"); }
    if bits & FEAT_INPUT_TOUCH   != 0 { names.push("input_touch"); }
    if bits & FEAT_INPUT_BUTTONS != 0 { names.push("input_buttons"); }
    if bits & FEAT_PSRAM         != 0 { names.push("psram"); }
    if bits & FEAT_BATTERY       != 0 { names.push("battery"); }
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
            ChipId::Esp32   => PLATFORM_ESP32,
            ChipId::Esp32S3 => PLATFORM_ESP32S3,
            ChipId::Rp2040  => PLATFORM_RP2040,
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
/// jumping to the boot-rom. Both crates must agree on this value.
/// Confirmed safe against ESP32 DRAM map: 0x3FFB_0000–0x3FFB_0007 (8 bytes).
pub const PLATFORM_PTR_ADDR: usize = 0x3FFB_0000;

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
}

/// Validate a parsed header byte slice.
///
/// - `device_features`: bitmask of what this device provides.
/// - `our_platform`: this device's platform byte (e.g. PLATFORM_ESP32).
/// - `flashpoint_current`: this firmware's API version.
/// - `flashpoint_last_breaking`: oldest API version still compatible with this firmware.
///
/// Returns `Ok(payload_start_offset)` on success. Checksum is verified separately
/// by the caller once the full payload is available.
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
    if data[OFF_PLATFORM] != our_platform {
        return Err(HeaderError::WrongPlatform);
    }
    let built_against = u32::from_le_bytes(
        data[OFF_BUILT_AGAINST..OFF_BUILT_AGAINST + 4].try_into().unwrap()
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
    if data.len() < hdr_size || data[hdr_size - 1] != HEADER_END_MAGIC {
        return Err(HeaderError::BadTerminator);
    }
    let required = u64::from_le_bytes(
        data[OFF_REQUIRED_FEATURES..OFF_REQUIRED_FEATURES + 8].try_into().unwrap()
    );
    if device_features & required != required {
        return Err(HeaderError::MissingFeatures);
    }
    let payload_len = u32::from_le_bytes(
        data[OFF_PAYLOAD_LEN..OFF_PAYLOAD_LEN + 4].try_into().unwrap()
    ) as usize;
    if payload_len == 0 {
        return Err(HeaderError::BadPayloadLen);
    }
    Ok(hdr_size) // payload starts here; checksum verified separately with payload bytes
}

/// Build a v1 header block (exactly HEADER_V1_SIZE bytes).
/// `checksum`: SHA-256 of the payload bytes (computed by caller).
/// `built_against`: Flashpoint API version this ROM was compiled for.
pub fn build_header(
    platform: u8,
    rom_version: [u8; 3],
    built_against: u32,
    flags: u16,
    required_features: u64,
    payload_len: u32,
    checksum: [u8; 32],
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
    h[OFF_CHECKSUM..OFF_CHECKSUM + 32].copy_from_slice(&checksum);
    h[OFF_HEADER_SIZE..OFF_HEADER_SIZE + 2]
        .copy_from_slice(&(HEADER_V1_SIZE as u16).to_le_bytes());
    // reserved bytes (0x3C..0x3F) remain zero
    h[OFF_HEADER_END] = HEADER_END_MAGIC;
    h
}

// ─── Platform HAL types ──────────────────────────────────────────────────────

/// A single scanline of pixel data (RGB565, width × 2 bytes).
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

/// Hardware abstraction contract.
/// firmware implements this for each supported board.
/// kernel calls only these methods — zero hardware code in kernel.
pub trait Platform {
    fn sd_read_sectors(&self, start: u32, buf: &mut [u8])  -> Result<(), PlatformError>;
    fn sd_write_sectors(&self, start: u32, buf: &[u8])     -> Result<(), PlatformError>;
    fn sd_sector_count(&self) -> u32;
    fn nvs_read(&self, ns: &str, key: &str)                -> Result<Vec<u8>, PlatformError>;
    fn nvs_write(&self, ns: &str, key: &str, val: &[u8])   -> Result<(), PlatformError>;
    fn nvs_delete(&self, ns: &str, key: &str)              -> Result<(), PlatformError>;
    fn display_flush(&self, buf: &FrameBuffer)             -> Result<(), PlatformError>;
    fn display_clear(&self)                                -> Result<(), PlatformError>;
    fn display_width(&self)  -> u16;
    fn display_height(&self) -> u16;
    fn poll_event(&self) -> Option<Event>;
    fn battery_percent(&self) -> u8;
    fn chip_id(&self)         -> ChipId;
    fn reboot(&self)          -> !;
    fn sleep_ms(&self, ms: u32);
    /// Returns the (current, last_breaking) Flashpoint API version of the running firmware.
    fn flashpoint_version(&self) -> (u32, u32);
}

// ─── Hardware-agnostic kernel entry ─────────────────────────────────────────

/// Kernel entry point callable by both real hardware (via Platform ptr handoff)
/// and the emulator (directly). Zero hardware code — only Platform trait calls.
pub fn boot_main(platform: &dyn Platform) -> ! {
    platform.display_clear().ok();

    let w = platform.display_width();
    let h = platform.display_height();
    let mut row = [0u8; 640]; // max width (320) × 2 bytes/pixel

    for y in 0..h {
        render_row(y, h, w, &mut row[..w as usize * 2]);
        platform.display_flush(&FrameBuffer {
            y,
            data: &row[..w as usize * 2],
        }).ok();
    }

    loop {
        if let Some(Event::BtnSelect) = platform.poll_event() {
            platform.reboot();
        }
        platform.sleep_ms(50);
    }
}

fn render_row(y: u16, h: u16, w: u16, row: &mut [u8]) {
    let text_top    = h * 2 / 5;
    let text_bottom = h * 3 / 5;
    let color: u16 = if y >= text_top && y < text_bottom { 0xFFFF } else { 0x000F };
    let bytes = color.to_le_bytes();
    for i in (0..w as usize * 2).step_by(2) {
        row[i]     = bytes[0];
        row[i + 1] = bytes[1];
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_checksum() -> [u8; 32] { [0xAB; 32] }

    fn make_valid_header() -> [u8; HEADER_V1_SIZE] {
        build_header(PLATFORM_ESP32, [0, 1, 0], FLASHPOINT_CURRENT, 0, 0, 1024, dummy_checksum())
    }

    #[test]
    fn round_trip_header_fields() {
        let h = make_valid_header();
        assert_eq!(&h[OFF_MAGIC..OFF_MAGIC + 4], &MAGIC);
        assert_eq!(h[OFF_PLATFORM], PLATFORM_ESP32);
        assert_eq!(h[OFF_HEADER_END], HEADER_END_MAGIC);
        assert_eq!(
            u16::from_le_bytes([h[OFF_HEADER_SIZE], h[OFF_HEADER_SIZE + 1]]),
            HEADER_V1_SIZE as u16
        );
        assert_eq!(
            u32::from_le_bytes(h[OFF_BUILT_AGAINST..OFF_BUILT_AGAINST + 4].try_into().unwrap()),
            FLASHPOINT_CURRENT
        );
    }

    #[test]
    fn validate_rejects_bad_magic() {
        let mut h = make_valid_header();
        h[0] = 0xFF;
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
            Err(HeaderError::BadMagic)
        );
    }

    #[test]
    fn validate_rejects_wrong_platform() {
        let h = make_valid_header();
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32S3, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
            Err(HeaderError::WrongPlatform)
        );
    }

    #[test]
    fn validate_rejects_api_incompatible_too_old() {
        let future_ver = version_pack(1, 0, 0);
        let h = build_header(PLATFORM_ESP32, [0, 1, 0], future_ver, 0, 0, 1024, dummy_checksum());
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
            Err(HeaderError::ApiIncompatible)
        );
    }

    #[test]
    fn validate_rejects_missing_features() {
        let h = build_header(PLATFORM_ESP32, [0, 1, 0], FLASHPOINT_CURRENT, 0, FEAT_PSRAM, 1024, dummy_checksum());
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
            Err(HeaderError::MissingFeatures)
        );
    }

    #[test]
    fn validate_passes_with_features_met() {
        let h = build_header(PLATFORM_ESP32, [0, 1, 0], FLASHPOINT_CURRENT, 0, FEAT_PSRAM, 1024, dummy_checksum());
        assert!(
            validate_header(&h, FEAT_PSRAM | FEAT_WIFI, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING).is_ok()
        );
    }

    #[test]
    fn validate_rejects_bad_terminator() {
        let mut h = make_valid_header();
        h[OFF_HEADER_END] = 0x00;
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
            Err(HeaderError::BadTerminator)
        );
    }

    #[test]
    fn parse_features_round_trip() {
        let bits = parse_features("psram,wifi,disp_tft").unwrap();
        assert_eq!(bits, FEAT_PSRAM | FEAT_WIFI | FEAT_DISP_TFT);
    }

    #[test]
    fn parse_features_unknown_returns_err() {
        assert!(parse_features("psram,unknownthing").is_err());
    }

    #[test]
    fn header_size_is_64() {
        assert_eq!(HEADER_V1_SIZE, 64);
        assert_eq!(OFF_HEADER_END, 63);
    }

    #[test]
    fn version_pack_unpack_round_trip() {
        let v = version_pack(1, 2, 3);
        assert_eq!(version_unpack(v), (1, 2, 3));
    }

    #[test]
    fn feature_flags_are_in_correct_bytes() {
        // connectivity in byte 0
        assert!(FEAT_WIFI < (1 << 8));
        assert!(FEAT_BLE  < (1 << 8));
        // display in byte 1
        assert!(FEAT_DISP_TFT  >= (1 << 8)  && FEAT_DISP_TFT  < (1 << 16));
        // input in byte 2
        assert!(FEAT_INPUT_TOUCH   >= (1 << 16) && FEAT_INPUT_TOUCH   < (1 << 24));
        assert!(FEAT_INPUT_BUTTONS >= (1 << 16) && FEAT_INPUT_BUTTONS < (1 << 24));
        // memory in byte 3
        assert!(FEAT_PSRAM   >= (1 << 24));
        assert!(FEAT_BATTERY >= (1 << 24));
    }
}
