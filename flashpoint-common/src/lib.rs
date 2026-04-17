#![cfg_attr(not(feature = "std"), no_std)]

// ─── Header constants ────────────────────────────────────────────────────────

pub const MAGIC: [u8; 6]          = *b"BROM\x00\x01";
pub const SPEC_VERSION: u16       = 1;
pub const HEADER_V1_SIZE: usize   = 64;
pub const HEADER_END_MAGIC: u8    = 0xFE;

// Byte offsets within the header block
pub const OFF_MAGIC:             usize = 0x00;
pub const OFF_SPEC_VERSION:      usize = 0x06;
pub const OFF_PLATFORM:          usize = 0x08;
pub const OFF_ROM_VERSION:       usize = 0x09;
pub const OFF_FLAGS:             usize = 0x0C;
pub const OFF_REQUIRED_FEATURES: usize = 0x0E;
pub const OFF_PAYLOAD_LEN:       usize = 0x16;
pub const OFF_CHECKSUM:          usize = 0x1A;
pub const OFF_HEADER_SIZE:       usize = 0x3A;
pub const OFF_RESERVED:          usize = 0x3C;
pub const OFF_HEADER_END:        usize = 0x3F;

// ─── Platform IDs ────────────────────────────────────────────────────────────

pub const PLATFORM_ESP32:   u8 = 0x01;
pub const PLATFORM_ESP32S3: u8 = 0x02;
pub const PLATFORM_RP2040:  u8 = 0x03;
pub const PLATFORM_MULTI:   u8 = 0xFF; // future: multi-platform rom

// ─── Feature flags ───────────────────────────────────────────────────────────

pub const FEAT_PSRAM:          u64 = 1 << 0;
pub const FEAT_WIFI:           u64 = 1 << 1;
pub const FEAT_BLE:            u64 = 1 << 2;
pub const FEAT_DISPLAY_TFT:    u64 = 1 << 3;
pub const FEAT_DISPLAY_EINK:   u64 = 1 << 4;
pub const FEAT_INPUT_TOUCH:    u64 = 1 << 5;
pub const FEAT_INPUT_BUTTONS:  u64 = 1 << 6;
pub const FEAT_BATTERY:        u64 = 1 << 7;
pub const FEAT_USB_OTG:        u64 = 1 << 8;

/// Parse a comma-separated feature string into a bitmask.
/// e.g. "psram,wifi,display_tft" → FEAT_PSRAM | FEAT_WIFI | FEAT_DISPLAY_TFT
pub fn parse_features(s: &str) -> Result<u64, &str> {
    let mut bits = 0u64;
    for part in s.split(',') {
        bits |= match part.trim() {
            "psram"         => FEAT_PSRAM,
            "wifi"          => FEAT_WIFI,
            "ble"           => FEAT_BLE,
            "display_tft"   => FEAT_DISPLAY_TFT,
            "display_eink"  => FEAT_DISPLAY_EINK,
            "input_touch"   => FEAT_INPUT_TOUCH,
            "input_buttons" => FEAT_INPUT_BUTTONS,
            "battery"       => FEAT_BATTERY,
            "usb_otg"       => FEAT_USB_OTG,
            other           => return Err(other),
        };
    }
    Ok(bits)
}

/// Human-readable list of feature names from a bitmask.
#[cfg(feature = "std")]
pub fn features_to_names(bits: u64) -> std::vec::Vec<&'static str> {
    let mut names = std::vec::Vec::new();
    if bits & FEAT_PSRAM         != 0 { names.push("psram"); }
    if bits & FEAT_WIFI          != 0 { names.push("wifi"); }
    if bits & FEAT_BLE           != 0 { names.push("ble"); }
    if bits & FEAT_DISPLAY_TFT   != 0 { names.push("display_tft"); }
    if bits & FEAT_DISPLAY_EINK  != 0 { names.push("display_eink"); }
    if bits & FEAT_INPUT_TOUCH   != 0 { names.push("input_touch"); }
    if bits & FEAT_INPUT_BUTTONS != 0 { names.push("input_buttons"); }
    if bits & FEAT_BATTERY       != 0 { names.push("battery"); }
    if bits & FEAT_USB_OTG       != 0 { names.push("usb_otg"); }
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
/// Chosen to be above the typical ESP32 stack and below the load region.
/// Confirmed safe against ESP32 DRAM map: 0x3FFB_0000–0x3FFB_0007 (8 bytes).
pub const PLATFORM_PTR_ADDR: usize = 0x3FFB_0000;

// ─── Header validation ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    TooShort,
    BadMagic,
    BadSpecVersion,
    WrongPlatform,
    BadTerminator,
    UnsupportedHeaderVersion,
    MissingFeatures,
    BadPayloadLen,
    BadChecksum,
}

/// Validate a parsed header byte slice against a payload slice.
/// `device_features`: bitmask of what this device provides.
/// `our_platform`: this device's platform byte (e.g. PLATFORM_ESP32).
///
/// Returns Ok(payload_start_offset) on success — caller uses this to locate
/// the payload within the file/buffer.
pub fn validate_header(
    data: &[u8],
    device_features: u64,
    our_platform: u8,
) -> Result<usize, HeaderError> {
    if data.len() < HEADER_V1_SIZE {
        return Err(HeaderError::TooShort);
    }
    if data[OFF_MAGIC..OFF_MAGIC + 6] != MAGIC {
        return Err(HeaderError::BadMagic);
    }
    let spec = u16::from_le_bytes([data[OFF_SPEC_VERSION], data[OFF_SPEC_VERSION + 1]]);
    if spec != SPEC_VERSION {
        return Err(HeaderError::BadSpecVersion);
    }
    if data[OFF_PLATFORM] != our_platform {
        return Err(HeaderError::WrongPlatform);
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
pub fn build_header(
    platform: u8,
    rom_version: [u8; 3],
    flags: u16,
    required_features: u64,
    payload_len: u32,
    checksum: [u8; 32],
) -> [u8; HEADER_V1_SIZE] {
    let mut h = [0u8; HEADER_V1_SIZE];
    h[OFF_MAGIC..OFF_MAGIC + 6].copy_from_slice(&MAGIC);
    h[OFF_SPEC_VERSION..OFF_SPEC_VERSION + 2].copy_from_slice(&SPEC_VERSION.to_le_bytes());
    h[OFF_PLATFORM] = platform;
    h[OFF_ROM_VERSION..OFF_ROM_VERSION + 3].copy_from_slice(&rom_version);
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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_checksum() -> [u8; 32] { [0xAB; 32] }

    fn make_valid_header() -> [u8; HEADER_V1_SIZE] {
        build_header(PLATFORM_ESP32, [0, 1, 0], 0, 0, 1024, dummy_checksum())
    }

    #[test]
    fn round_trip_header_fields() {
        let h = make_valid_header();
        assert_eq!(&h[OFF_MAGIC..OFF_MAGIC+6], &MAGIC);
        assert_eq!(u16::from_le_bytes([h[OFF_SPEC_VERSION], h[OFF_SPEC_VERSION+1]]), 1);
        assert_eq!(h[OFF_PLATFORM], PLATFORM_ESP32);
        assert_eq!(h[OFF_HEADER_END], HEADER_END_MAGIC);
        assert_eq!(
            u16::from_le_bytes([h[OFF_HEADER_SIZE], h[OFF_HEADER_SIZE+1]]),
            HEADER_V1_SIZE as u16
        );
    }

    #[test]
    fn validate_rejects_bad_magic() {
        let mut h = make_valid_header();
        h[0] = 0xFF;
        assert_eq!(validate_header(&h, 0, PLATFORM_ESP32), Err(HeaderError::BadMagic));
    }

    #[test]
    fn validate_rejects_wrong_platform() {
        let h = make_valid_header();
        assert_eq!(validate_header(&h, 0, PLATFORM_ESP32S3), Err(HeaderError::WrongPlatform));
    }

    #[test]
    fn validate_rejects_missing_features() {
        let h = build_header(PLATFORM_ESP32, [0,1,0], 0, FEAT_PSRAM, 1024, dummy_checksum());
        assert_eq!(
            validate_header(&h, 0 /* no PSRAM */, PLATFORM_ESP32),
            Err(HeaderError::MissingFeatures)
        );
    }

    #[test]
    fn validate_passes_with_features_met() {
        let h = build_header(PLATFORM_ESP32, [0,1,0], 0, FEAT_PSRAM, 1024, dummy_checksum());
        assert!(validate_header(&h, FEAT_PSRAM | FEAT_WIFI, PLATFORM_ESP32).is_ok());
    }

    #[test]
    fn validate_rejects_bad_terminator() {
        let mut h = make_valid_header();
        h[OFF_HEADER_END] = 0x00;
        assert_eq!(validate_header(&h, 0, PLATFORM_ESP32), Err(HeaderError::BadTerminator));
    }

    #[test]
    fn parse_features_round_trip() {
        let bits = parse_features("psram,wifi,display_tft").unwrap();
        assert_eq!(bits, FEAT_PSRAM | FEAT_WIFI | FEAT_DISPLAY_TFT);
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
}
