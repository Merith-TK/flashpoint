use crate::constants::*;
use crate::crc::crc32;
use crate::error::HeaderError;
use crate::types::PayloadType;

pub const fn version_pack(major: u8, minor: u8, patch: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (patch as u32)
}

pub fn version_unpack(v: u32) -> (u8, u8, u8) {
    ((v >> 16) as u8, ((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8)
}

pub const FLASHPOINT_CURRENT: u32 = version_pack(0, 2, 0);
pub const FLASHPOINT_LAST_BREAKING: u32 = version_pack(0, 2, 0);

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

    let id_bytes = rom_id.as_bytes();
    let copy_len = id_bytes.len().min(ROM_ID_LEN - 1);
    h[OFF_ROM_ID..OFF_ROM_ID + copy_len].copy_from_slice(&id_bytes[..copy_len]);

    h[OFF_COMPAT_PLATFORMS..OFF_COMPAT_PLATFORMS + 3].copy_from_slice(&compat_platforms);
    h[OFF_HEADER_SIZE..OFF_HEADER_SIZE + 2].copy_from_slice(&(HEADER_V1_SIZE as u16).to_le_bytes());
    h[OFF_HEADER_END..OFF_HEADER_END + 4].copy_from_slice(&HEADER_END_MAGIC);
    h
}
