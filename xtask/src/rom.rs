use std::fs;
use std::io::Write;
use std::path::Path;
use sha2::{Sha256, Digest};
use common::*;

// ─── pack ────────────────────────────────────────────────────────────────────

pub fn do_pack(
    platform_str: &str,
    version_str: &str,
    built_against_str: Option<&str>,
    requires_str: Option<&str>,
    compress: bool,
    input: &Path,
    output: &Path,
) -> Result<(), String> {
    let platform = parse_platform(platform_str)?;
    let rom_version = parse_version(version_str)?;
    let built_against = match built_against_str {
        Some(s) => {
            let v = parse_version(s)?;
            version_pack(v[0], v[1], v[2])
        }
        None => FLASHPOINT_CURRENT,
    };
    let required_features = match requires_str {
        Some(s) => parse_features(s).map_err(|e| format!("unknown feature: '{e}'"))?,
        None => 0,
    };
    let flags: u16 = if compress { 0x0001 } else { 0 };

    let payload = fs::read(input)
        .map_err(|e| format!("failed to read '{}': {e}", input.display()))?;
    if payload.is_empty() {
        return Err("payload is empty".into());
    }
    if payload.len() > u32::MAX as usize {
        return Err("payload exceeds 4 GB".into());
    }

    let checksum: [u8; 32] = Sha256::digest(&payload).into();
    let header = build_header(platform, rom_version, built_against, flags,
        required_features, payload.len() as u32, checksum);

    let mut out = fs::File::create(output)
        .map_err(|e| format!("failed to create '{}': {e}", output.display()))?;
    out.write_all(&header).map_err(|e| e.to_string())?;
    out.write_all(&payload).map_err(|e| e.to_string())?;

    let (ma, mi, pa) = version_unpack(built_against);
    println!("wrote {} ({} header + {} payload)",
        output.display(), HEADER_V1_SIZE, payload.len());
    println!("built-against: {ma}.{mi}.{pa}");
    println!("sha256: {}", hex::encode(checksum));
    Ok(())
}

// ─── verify ──────────────────────────────────────────────────────────────────

pub fn do_verify(input: &Path) -> Result<(), String> {
    let data = fs::read(input)
        .map_err(|e| format!("failed to read '{}': {e}", input.display()))?;

    if data.len() < HEADER_V1_SIZE {
        return Err(format!("file too short: {} bytes", data.len()));
    }

    let hdr = &data[..HEADER_V1_SIZE];

    println!("=== flashpoint.rom verification ===");
    println!("file:     {}", input.display());
    println!("size:     {} bytes", data.len());
    println!();

    let magic_ok = hdr[OFF_MAGIC..OFF_MAGIC + 4] == MAGIC;
    println!("magic:          {} {}", hex::encode(&hdr[OFF_MAGIC..OFF_MAGIC + 4]), tick(magic_ok));

    let plat = hdr[OFF_PLATFORM];
    println!("platform:       0x{plat:02X} ({})", platform_name(plat));

    let v = &hdr[OFF_ROM_VERSION..OFF_ROM_VERSION + 3];
    println!("rom_version:    {}.{}.{}", v[0], v[1], v[2]);

    let built_against = u32::from_le_bytes(hdr[OFF_BUILT_AGAINST..OFF_BUILT_AGAINST + 4].try_into().unwrap());
    let (ma, mi, pa) = version_unpack(built_against);
    let (cur_ma, cur_mi, cur_pa) = version_unpack(FLASHPOINT_CURRENT);
    let (brk_ma, brk_mi, brk_pa) = version_unpack(FLASHPOINT_LAST_BREAKING);
    let api_ok = built_against >= FLASHPOINT_LAST_BREAKING && built_against <= FLASHPOINT_CURRENT;
    println!("built_against:  {ma}.{mi}.{pa} (firmware: {cur_ma}.{cur_mi}.{cur_pa}, min: {brk_ma}.{brk_mi}.{brk_pa}) {}",
        tick(api_ok));

    let flags = u16::from_le_bytes([hdr[OFF_FLAGS], hdr[OFF_FLAGS + 1]]);
    println!("flags:          0x{flags:04X}{}", if flags & 1 != 0 { " [compressed]" } else { "" });

    let req = u64::from_le_bytes(hdr[OFF_REQUIRED_FEATURES..OFF_REQUIRED_FEATURES + 8].try_into().unwrap());
    let feat_names = features_to_names(req);
    println!("required:       0x{req:016X} [{}]",
        if feat_names.is_empty() { "none".into() } else { feat_names.join(", ") });

    let payload_len = u32::from_le_bytes(hdr[OFF_PAYLOAD_LEN..OFF_PAYLOAD_LEN + 4].try_into().unwrap()) as usize;
    let payload_present = data.len() >= HEADER_V1_SIZE + payload_len;
    println!("payload_len:    {} bytes {}", payload_len, tick(payload_present));

    let hdr_size = u16::from_le_bytes([hdr[OFF_HEADER_SIZE], hdr[OFF_HEADER_SIZE + 1]]) as usize;
    println!("header_size:    {} {}", hdr_size, tick(hdr_size == HEADER_V1_SIZE));

    let term_ok = hdr[OFF_HEADER_END] == HEADER_END_MAGIC;
    println!("header_end:     0x{:02X} {}", hdr[OFF_HEADER_END], tick(term_ok));

    println!();
    let stored = &hdr[OFF_CHECKSUM..OFF_CHECKSUM + 32];
    println!("stored sha256:   {}", hex::encode(stored));

    if !payload_present || payload_len == 0 {
        println!("\nRESULT: INCOMPLETE");
        return Err("payload missing or truncated".into());
    }

    let payload = &data[hdr_size..hdr_size + payload_len];
    let actual: [u8; 32] = Sha256::digest(payload).into();
    let checksum_ok = actual == stored;
    println!("computed sha256: {} {}", hex::encode(actual), tick(checksum_ok));

    let all_ok = magic_ok && api_ok && term_ok && hdr_size == HEADER_V1_SIZE && checksum_ok;

    println!("\nRESULT: {}", if all_ok { "VALID" } else { "INVALID" });
    if all_ok { Ok(()) } else { Err("verification failed".into()) }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn parse_platform(s: &str) -> Result<u8, String> {
    match s.to_lowercase().as_str() {
        "esp32"              => Ok(PLATFORM_ESP32),
        "esp32-s3"|"esp32s3" => Ok(PLATFORM_ESP32S3),
        "rp2040"             => Ok(PLATFORM_RP2040),
        other => Err(format!("unknown platform '{other}': use esp32 | esp32-s3 | rp2040")),
    }
}

pub fn parse_version(s: &str) -> Result<[u8; 3], String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("version must be X.Y.Z, got '{s}'"));
    }
    let mut out = [0u8; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p.parse::<u8>()
            .map_err(|_| format!("version component '{p}' must be 0–255"))?;
    }
    Ok(out)
}

fn platform_name(b: u8) -> &'static str {
    match b {
        PLATFORM_ESP32   => "esp32",
        PLATFORM_ESP32S3 => "esp32-s3",
        PLATFORM_RP2040  => "rp2040",
        PLATFORM_MULTI   => "multi",
        _                => "unknown",
    }
}

fn tick(ok: bool) -> &'static str { if ok { "✓" } else { "✗" } }

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn write_tmp(data: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(data).unwrap();
        f
    }

    #[test]
    fn pack_and_verify_round_trip() {
        let input  = write_tmp(&vec![0xDEu8; 256]);
        let output = NamedTempFile::new().unwrap();
        do_pack("esp32", "0.1.0", None, None, false, input.path(), output.path()).unwrap();
        do_verify(output.path()).unwrap();
    }

    #[test]
    fn output_size_is_header_plus_payload() {
        let input  = write_tmp(&vec![0u8; 1000]);
        let output = NamedTempFile::new().unwrap();
        do_pack("esp32", "1.2.3", None, None, false, input.path(), output.path()).unwrap();
        assert_eq!(fs::read(output.path()).unwrap().len(), HEADER_V1_SIZE + 1000);
    }

    #[test]
    fn features_round_trip() {
        let input  = write_tmp(&vec![0xAAu8; 64]);
        let output = NamedTempFile::new().unwrap();
        do_pack("esp32", "0.1.0", None, Some("psram,wifi"), false, input.path(), output.path()).unwrap();
        let data = fs::read(output.path()).unwrap();
        let stored = u64::from_le_bytes(data[OFF_REQUIRED_FEATURES..OFF_REQUIRED_FEATURES + 8].try_into().unwrap());
        assert_eq!(stored, FEAT_PSRAM | FEAT_WIFI);
    }

    #[test]
    fn magic_is_flpt() {
        let input  = write_tmp(&vec![0u8; 64]);
        let output = NamedTempFile::new().unwrap();
        do_pack("esp32", "0.1.0", None, None, false, input.path(), output.path()).unwrap();
        let data = fs::read(output.path()).unwrap();
        assert_eq!(&data[OFF_MAGIC..OFF_MAGIC + 4], b"FLPT");
    }

    #[test]
    fn built_against_defaults_to_current() {
        let input  = write_tmp(&vec![0u8; 64]);
        let output = NamedTempFile::new().unwrap();
        do_pack("esp32", "0.1.0", None, None, false, input.path(), output.path()).unwrap();
        let data = fs::read(output.path()).unwrap();
        let stored = u32::from_le_bytes(data[OFF_BUILT_AGAINST..OFF_BUILT_AGAINST + 4].try_into().unwrap());
        assert_eq!(stored, FLASHPOINT_CURRENT);
    }

    #[test]
    fn verify_rejects_corrupt_magic() {
        let input  = write_tmp(&vec![0u8; 64]);
        let output = NamedTempFile::new().unwrap();
        do_pack("esp32", "0.1.0", None, None, false, input.path(), output.path()).unwrap();
        let mut data = fs::read(output.path()).unwrap();
        data[0] = 0xFF;
        let corrupted = write_tmp(&data);
        assert!(do_verify(corrupted.path()).is_err());
    }

    #[test]
    fn pack_rejects_empty_input() {
        let input  = write_tmp(&[]);
        let output = NamedTempFile::new().unwrap();
        assert!(do_pack("esp32", "0.1.0", None, None, false, input.path(), output.path()).is_err());
    }

    #[test]
    fn pack_rejects_bad_platform() {
        let input  = write_tmp(&[1, 2, 3]);
        let output = NamedTempFile::new().unwrap();
        assert!(do_pack("esp999", "0.1.0", None, None, false, input.path(), output.path()).is_err());
    }

    #[test]
    fn version_components_must_be_0_to_255() {
        let input  = write_tmp(&[1, 2, 3]);
        let output = NamedTempFile::new().unwrap();
        assert!(do_pack("esp32", "0.256.0", None, None, false, input.path(), output.path()).is_err());
    }
}
