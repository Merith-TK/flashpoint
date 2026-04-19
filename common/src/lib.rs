#![cfg_attr(not(feature = "std"), no_std)]

use crc::{Crc, CRC_32_ISO_HDLC};

#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

// ─── Header constants ────────────────────────────────────────────────────────

pub const MAGIC:            [u8; 4] = *b"FLPT";
pub const HEADER_END_MAGIC: [u8; 4] = *b"FLPE";
pub const HEADER_V1_SIZE:   usize   = 64;

// Byte offsets within the v2 header block (64 bytes total)
pub const OFF_MAGIC:             usize = 0x00; // 4 bytes  "FLPT"
pub const OFF_PLATFORM:          usize = 0x04; // 1 byte   primary platform
pub const OFF_ROM_VERSION:       usize = 0x05; // 3 bytes  [major, minor, patch]
pub const OFF_BUILT_AGAINST:     usize = 0x08; // 4 bytes  Flashpoint API version (LE u32)
pub const OFF_FLAGS:             usize = 0x0C; // 2 bytes
pub const OFF_REQUIRED_FEATURES: usize = 0x0E; // 8 bytes  hardware bitmask (LE u64)
pub const OFF_PAYLOAD_LEN:       usize = 0x16; // 4 bytes  (LE u32)
pub const OFF_CRC32:             usize = 0x1A; // 4 bytes  CRC32 of payload (LE u32)
pub const OFF_PAYLOAD_TYPE:      usize = 0x1E; // 1 byte   PayloadType
pub const OFF_ROM_ID:            usize = 0x1F; // 24 bytes null-terminated ASCII namespace
pub const OFF_COMPAT_PLATFORMS:  usize = 0x37; // 3 bytes  additional supported platforms
pub const OFF_HEADER_SIZE:       usize = 0x3A; // 2 bytes  (LE u16), always 64
pub const OFF_HEADER_END:        usize = 0x3C; // 4 bytes  "FLPE"

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
            _    => None,
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

pub const FLASHPOINT_CURRENT:       u32 = version_pack(0, 2, 0); // bumped for v2 header
pub const FLASHPOINT_LAST_BREAKING: u32 = version_pack(0, 2, 0);

// ─── Platform IDs ────────────────────────────────────────────────────────────

pub const PLATFORM_ESP32:   u8 = 0x01;
pub const PLATFORM_ESP32S3: u8 = 0x02;
pub const PLATFORM_RP2040:  u8 = 0x03;
pub const PLATFORM_ANY:     u8 = 0xFF; // wildcard: ROM runs on any platform

// ─── Feature flags (byte-grouped u64) ───────────────────────────────────────

// Byte 0 — Connectivity
pub const FEAT_WIFI:          u64 = 1 << 0;
pub const FEAT_BLE:           u64 = 1 << 1;
pub const FEAT_USB_OTG:       u64 = 1 << 2;

// Byte 1 — Display
pub const FEAT_DISP_TFT:      u64 = 1 << 8;
pub const FEAT_DISP_EINK:     u64 = 1 << 9;

// Byte 2 — Input
pub const FEAT_INPUT_TOUCH:   u64 = 1 << 16;
pub const FEAT_INPUT_BUTTONS: u64 = 1 << 17;

// Byte 3 — Memory / Power
pub const FEAT_PSRAM:         u64 = 1 << 24;
pub const FEAT_BATTERY:       u64 = 1 << 25;

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
pub enum ChipId { Esp32, Esp32S3, Rp2040 }

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
    BtnUp, BtnDown, BtnLeft, BtnRight, BtnSelect, BtnBack,
    BatteryLow, HibernateWarning,
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
    let compat  = &data[OFF_COMPAT_PLATFORMS..OFF_COMPAT_PLATFORMS + 3];
    let platform_ok = primary == PLATFORM_ANY
        || primary == our_platform
        || compat.iter().any(|&b| b != 0x00 && (b == our_platform || b == PLATFORM_ANY));
    if !platform_ok {
        return Err(HeaderError::WrongPlatform);
    }

    let built_against = u32::from_le_bytes(
        data[OFF_BUILT_AGAINST..OFF_BUILT_AGAINST + 4].try_into().unwrap()
    );
    if built_against < flashpoint_last_breaking || built_against > flashpoint_current {
        return Err(HeaderError::ApiIncompatible);
    }

    let hdr_size = u16::from_le_bytes(
        [data[OFF_HEADER_SIZE], data[OFF_HEADER_SIZE + 1]]
    ) as usize;
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
    let expected = u32::from_le_bytes(
        header[OFF_CRC32..OFF_CRC32 + 4].try_into().unwrap()
    );
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
    h[OFF_HEADER_SIZE..OFF_HEADER_SIZE + 2]
        .copy_from_slice(&(HEADER_V1_SIZE as u16).to_le_bytes());
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
    SdReadError, SdWriteError, NvsError, DisplayError, NotSupported,
}

/// Hardware abstraction contract. firmware implements this per board.
/// kernel / WASM host API calls only these methods — zero hardware code elsewhere.
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
    fn flashpoint_version(&self) -> (u32, u32);
    /// Max bytes of WASM linear memory this device can provide (0 = no WASM support)
    fn wasm_arena_limit(&self) -> usize;
    /// Max bytes of Lua heap this device can provide (0 = no Lua support)
    fn lua_heap_limit(&self) -> usize;
}

// ─── Hardware-agnostic kernel entry ─────────────────────────────────────────

pub fn boot_main(platform: &dyn Platform) -> ! {
    platform.display_clear().ok();

    let w = platform.display_width();
    let h = platform.display_height();
    let mut row = [0u8; 640];

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

    fn dummy_crc() -> u32 { 0xDEAD_BEEF }

    fn make_valid_header() -> [u8; HEADER_V1_SIZE] {
        let payload = b"test payload";
        let checksum = crc32(payload);
        build_header(
            PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, 0,
            payload.len() as u32, PayloadType::Native, "com.test", [0, 0, 0], checksum,
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
    fn validate_accepts_compat_platform() {
        // Build ROM targeting ESP32S3 primary + ESP32 as compat
        let payload = b"x";
        let h = build_header(
            PLATFORM_ESP32S3, [0, 2, 0], FLASHPOINT_CURRENT, 0, 0,
            1, PayloadType::Native, "", [PLATFORM_ESP32, 0, 0], crc32(payload),
        );
        assert!(validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING).is_ok());
    }

    #[test]
    fn validate_accepts_platform_any_wildcard() {
        let payload = b"x";
        let h = build_header(
            PLATFORM_ANY, [0, 2, 0], FLASHPOINT_CURRENT, 0, 0,
            1, PayloadType::Native, "", [0, 0, 0], crc32(payload),
        );
        assert!(validate_header(&h, 0, PLATFORM_RP2040, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING).is_ok());
    }

    #[test]
    fn validate_rejects_api_incompatible_too_old() {
        let future_ver = version_pack(1, 0, 0);
        let h = build_header(
            PLATFORM_ESP32, [0, 2, 0], future_ver, 0, 0,
            1, PayloadType::Native, "", [0, 0, 0], dummy_crc(),
        );
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
            Err(HeaderError::ApiIncompatible)
        );
    }

    #[test]
    fn validate_rejects_missing_features() {
        let h = build_header(
            PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, FEAT_PSRAM,
            1, PayloadType::Native, "", [0, 0, 0], dummy_crc(),
        );
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
            Err(HeaderError::MissingFeatures)
        );
    }

    #[test]
    fn validate_passes_with_features_met() {
        let h = build_header(
            PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, FEAT_PSRAM,
            1, PayloadType::Native, "", [0, 0, 0], dummy_crc(),
        );
        assert!(validate_header(&h, FEAT_PSRAM | FEAT_WIFI, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING).is_ok());
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
    fn verify_crc32_accepts_correct_payload() {
        let payload = b"hello flashpoint";
        let checksum = crc32(payload);
        let h = build_header(
            PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, 0,
            payload.len() as u32, PayloadType::Native, "", [0, 0, 0], checksum,
        );
        assert!(verify_crc32(&h, payload).is_ok());
    }

    #[test]
    fn verify_crc32_rejects_corrupted_payload() {
        let payload = b"hello flashpoint";
        let checksum = crc32(payload);
        let h = build_header(
            PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, 0,
            payload.len() as u32, PayloadType::Native, "", [0, 0, 0], checksum,
        );
        assert_eq!(verify_crc32(&h, b"hello flashpointX"), Err(HeaderError::BadChecksum));
    }

    #[test]
    fn validate_rejects_unknown_payload_type() {
        let mut h = make_valid_header();
        h[OFF_PAYLOAD_TYPE] = 0xFF;
        assert_eq!(
            validate_header(&h, 0, PLATFORM_ESP32, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING),
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
            PLATFORM_ESP32, [0, 2, 0], FLASHPOINT_CURRENT, 0, 0,
            1, PayloadType::Wasm32, long_id, [0, 0, 0], crc32(payload),
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
