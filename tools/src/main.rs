use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use sha2::{Sha256, Digest};
use flashpoint_common::*;

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "mkrom", about = "Flashpoint ROM tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Wrap a raw binary with a Flashpoint header → flashpoint.rom
    Pack {
        /// Target platform: esp32 | esp32-s3 | rp2040
        #[arg(long)]
        platform: String,

        /// ROM semantic version: X.Y.Z
        #[arg(long)]
        version: String,

        /// Comma-separated required hardware features
        /// e.g. --requires psram,wifi,display_tft
        #[arg(long)]
        requires: Option<String>,

        /// Set the compressed flag (future use — does not compress)
        #[arg(long, default_value_t = false)]
        compress: bool,

        /// Input raw binary
        input: PathBuf,

        /// Output .rom file
        output: PathBuf,
    },
    /// Parse and validate a flashpoint.rom file
    Verify {
        /// .rom file to inspect
        input: PathBuf,
    },
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Pack { platform, version, requires, compress, input, output } => {
            if let Err(e) = cmd_pack(&platform, &version, requires.as_deref(), compress, &input, &output) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        Command::Verify { input } => {
            if let Err(e) = cmd_verify(&input) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }
}

// ─── pack ────────────────────────────────────────────────────────────────────

fn cmd_pack(
    platform_str: &str,
    version_str: &str,
    requires_str: Option<&str>,
    compress: bool,
    input: &Path,
    output: &Path,
) -> Result<(), String> {
    let platform = parse_platform(platform_str)?;
    let rom_version = parse_version(version_str)?;
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
    let header = build_header(
        platform,
        rom_version,
        flags,
        required_features,
        payload.len() as u32,
        checksum,
    );

    let mut out = fs::File::create(output)
        .map_err(|e| format!("failed to create '{}': {e}", output.display()))?;
    out.write_all(&header).map_err(|e| e.to_string())?;
    out.write_all(&payload).map_err(|e| e.to_string())?;

    let total = HEADER_V1_SIZE + payload.len();
    println!("wrote {} ({} header + {} payload)", output.display(), HEADER_V1_SIZE, payload.len());
    println!("total: {} bytes", total);
    println!("sha256: {}", hex::encode(checksum));

    Ok(())
}

// ─── verify ──────────────────────────────────────────────────────────────────

fn cmd_verify(input: &Path) -> Result<(), String> {
    let data = fs::read(input)
        .map_err(|e| format!("failed to read '{}': {e}", input.display()))?;

    if data.len() < HEADER_V1_SIZE {
        return Err(format!("file too short: {} bytes (need at least {})", data.len(), HEADER_V1_SIZE));
    }

    let hdr = &data[..HEADER_V1_SIZE];

    println!("=== flashpoint.rom verification ===");
    println!("file:         {}", input.display());
    println!("file size:    {} bytes", data.len());
    println!();

    // magic
    let magic_ok = hdr[OFF_MAGIC..OFF_MAGIC+6] == MAGIC;
    println!("magic:        {} {}",
        hex::encode(&hdr[OFF_MAGIC..OFF_MAGIC+6]),
        pass_fail(magic_ok));

    // spec version
    let spec = u16::from_le_bytes([hdr[OFF_SPEC_VERSION], hdr[OFF_SPEC_VERSION+1]]);
    println!("spec_version: {} {}", spec, pass_fail(spec == SPEC_VERSION));

    // platform
    let plat = hdr[OFF_PLATFORM];
    println!("platform:     0x{:02X} ({})", plat, platform_name(plat));

    // rom version
    let v = &hdr[OFF_ROM_VERSION..OFF_ROM_VERSION+3];
    println!("rom_version:  {}.{}.{}", v[0], v[1], v[2]);

    // flags
    let flags = u16::from_le_bytes([hdr[OFF_FLAGS], hdr[OFF_FLAGS+1]]);
    println!("flags:        0x{:04X}{}", flags, if flags & 1 != 0 { " [compressed]" } else { "" });

    // required_features
    let req = u64::from_le_bytes(hdr[OFF_REQUIRED_FEATURES..OFF_REQUIRED_FEATURES+8].try_into().unwrap());
    let feat_names = features_to_names(req);
    println!("required_features: 0x{:016X} [{}]",
        req,
        if feat_names.is_empty() { "none".to_string() } else { feat_names.join(", ") });

    // payload_len
    let payload_len = u32::from_le_bytes(hdr[OFF_PAYLOAD_LEN..OFF_PAYLOAD_LEN+4].try_into().unwrap()) as usize;
    let payload_present = data.len() >= HEADER_V1_SIZE + payload_len;
    println!("payload_len:  {} bytes {}", payload_len, pass_fail(payload_present));

    // header_size
    let hdr_size = u16::from_le_bytes([hdr[OFF_HEADER_SIZE], hdr[OFF_HEADER_SIZE+1]]) as usize;
    println!("header_size:  {} {}", hdr_size, pass_fail(hdr_size == HEADER_V1_SIZE));

    // header_end terminator
    let term_ok = hdr[OFF_HEADER_END] == HEADER_END_MAGIC;
    println!("header_end:   0x{:02X} {}", hdr[OFF_HEADER_END], pass_fail(term_ok));

    // checksum
    println!();
    let stored_checksum = &hdr[OFF_CHECKSUM..OFF_CHECKSUM+32];
    println!("stored sha256:   {}", hex::encode(stored_checksum));

    if payload_present && payload_len > 0 {
        let payload = &data[hdr_size..hdr_size + payload_len];
        let actual: [u8; 32] = Sha256::digest(payload).into();
        let checksum_ok = actual == stored_checksum;
        println!("computed sha256: {} {}", hex::encode(actual), pass_fail(checksum_ok));

        println!();
        let all_ok = magic_ok && spec == SPEC_VERSION && term_ok
            && hdr_size == HEADER_V1_SIZE && payload_present && checksum_ok;
        if all_ok {
            println!("RESULT: VALID");
        } else {
            println!("RESULT: INVALID");
            return Err("verification failed".into());
        }
    } else {
        println!();
        println!("RESULT: INCOMPLETE (payload missing or truncated)");
        return Err("payload missing or truncated".into());
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_platform(s: &str) -> Result<u8, String> {
    match s.to_lowercase().as_str() {
        "esp32"    => Ok(PLATFORM_ESP32),
        "esp32-s3" | "esp32s3" => Ok(PLATFORM_ESP32S3),
        "rp2040"   => Ok(PLATFORM_RP2040),
        other      => Err(format!("unknown platform '{other}': use esp32 | esp32-s3 | rp2040")),
    }
}

fn parse_version(s: &str) -> Result<[u8; 3], String> {
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
        PLATFORM_MULTI   => "multi (future)",
        _                => "unknown",
    }
}

fn pass_fail(ok: bool) -> &'static str {
    if ok { "✓" } else { "✗" }
}

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
        let payload = vec![0xDEu8; 256];
        let input  = write_tmp(&payload);
        let output = NamedTempFile::new().unwrap();

        cmd_pack("esp32", "0.1.0", None, false, input.path(), output.path()).unwrap();

        let data = fs::read(output.path()).unwrap();
        assert_eq!(data.len(), HEADER_V1_SIZE + payload.len());
        assert_eq!(data[OFF_HEADER_END], HEADER_END_MAGIC);

        cmd_verify(output.path()).unwrap();
    }

    #[test]
    fn output_size_is_header_plus_payload() {
        let payload = vec![0u8; 1000];
        let input  = write_tmp(&payload);
        let output = NamedTempFile::new().unwrap();
        cmd_pack("esp32", "1.2.3", None, false, input.path(), output.path()).unwrap();
        let data = fs::read(output.path()).unwrap();
        assert_eq!(data.len(), HEADER_V1_SIZE + 1000);
    }

    #[test]
    fn features_round_trip() {
        let payload = vec![0xAAu8; 64];
        let input  = write_tmp(&payload);
        let output = NamedTempFile::new().unwrap();
        cmd_pack("esp32", "0.1.0", Some("psram,wifi"), false, input.path(), output.path()).unwrap();
        let data = fs::read(output.path()).unwrap();
        let stored = u64::from_le_bytes(data[OFF_REQUIRED_FEATURES..OFF_REQUIRED_FEATURES+8].try_into().unwrap());
        assert_eq!(stored, FEAT_PSRAM | FEAT_WIFI);
    }

    #[test]
    fn verify_rejects_corrupt_magic() {
        let payload = vec![0u8; 64];
        let input  = write_tmp(&payload);
        let output = NamedTempFile::new().unwrap();
        cmd_pack("esp32", "0.1.0", None, false, input.path(), output.path()).unwrap();
        let mut data = fs::read(output.path()).unwrap();
        data[0] = 0xFF;
        let corrupted = write_tmp(&data);
        assert!(cmd_verify(corrupted.path()).is_err());
    }

    #[test]
    fn pack_rejects_empty_input() {
        let input  = write_tmp(&[]);
        let output = NamedTempFile::new().unwrap();
        assert!(cmd_pack("esp32", "0.1.0", None, false, input.path(), output.path()).is_err());
    }

    #[test]
    fn pack_rejects_bad_platform() {
        let input  = write_tmp(&[1,2,3]);
        let output = NamedTempFile::new().unwrap();
        assert!(cmd_pack("esp999", "0.1.0", None, false, input.path(), output.path()).is_err());
    }

    #[test]
    fn version_components_must_be_0_to_255() {
        let input  = write_tmp(&[1,2,3]);
        let output = NamedTempFile::new().unwrap();
        assert!(cmd_pack("esp32", "0.256.0", None, false, input.path(), output.path()).is_err());
    }
}
