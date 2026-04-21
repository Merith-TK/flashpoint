use common::*;

pub fn try_boot_from_buffer(buf: &[u8]) -> Result<u32, HeaderError> {
    let offset = validate_header(
        buf,
        crate::DEVICE_FEATURES,
        PLATFORM_ESP32,
        FLASHPOINT_CURRENT,
        FLASHPOINT_LAST_BREAKING,
    )?;
    Ok(sd_load_addr() + offset as u32)
}

pub fn flash_xip_addr(offset: u32) -> u32 {
    0x400C_0000 + offset
}
pub fn sd_load_addr() -> u32 {
    0x3FFB_8000
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{build_header, PayloadType};

    #[test]
    fn try_boot_rejects_bad_magic() {
        let buf = [0u8; HEADER_V1_SIZE];
        assert!(try_boot_from_buffer(&buf).is_err());
    }

    #[test]
    fn try_boot_accepts_valid_header() {
        let hdr = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            0,
            64,
            PayloadType::Native,
            "",
            [0, 0, 0],
            0,
        );
        assert!(try_boot_from_buffer(&hdr).is_ok());
    }

    #[test]
    fn try_boot_rejects_feature_mismatch() {
        let hdr = build_header(
            PLATFORM_ESP32,
            [0, 2, 0],
            FLASHPOINT_CURRENT,
            0,
            FEAT_PSRAM,
            64,
            PayloadType::Native,
            "",
            [0, 0, 0],
            0,
        );
        assert_eq!(
            try_boot_from_buffer(&hdr),
            Err(HeaderError::MissingFeatures)
        );
    }
}
