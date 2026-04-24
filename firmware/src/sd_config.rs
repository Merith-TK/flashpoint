/// SD-backed tinykv configuration store.
///
/// Keys are stored in JSON files on the SD card root: `nvs-<namespace>.bin`.
/// Values are hex-encoded byte strings so that both UTF-8 text (e.g. "wasm")
/// and arbitrary binary (touch calibration) are handled uniformly.
///
/// Only compiled for `board-cyd` because `tinykv` is a board-cyd dependency.

use common::{FileSystem, PlatformError};
use std::string::String;
use std::vec::Vec;
use tinykv::TinyKV;

// ─── Path helper ─────────────────────────────────────────────────────────────

fn nvs_path(ns: &str) -> String {
    // Flat file in SD root volume.  Subdirectory support would require
    // extending the FileSystem trait with directory-open operations.
    let mut s = String::from("nvs-");
    s.push_str(ns);
    s.push_str(".bin");
    s
}

// ─── Hex encode / decode ─────────────────────────────────────────────────────

fn hex_encode(b: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for byte in b {
        s.push(HEX[(byte >> 4) as usize] as char);
        s.push(HEX[(byte & 0xf) as usize] as char);
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut chars = s.chars();
    while let Some(hi) = chars.next() {
        let lo = chars.next()?;
        let hi = hi.to_digit(16)? as u8;
        let lo = lo.to_digit(16)? as u8;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

// ─── Load / save store ───────────────────────────────────────────────────────

fn load_store(fs: &mut dyn FileSystem, ns: &str) -> TinyKV {
    let path = nvs_path(ns);
    if !fs.exists(&path) {
        return TinyKV::new();
    }
    (|| -> Option<TinyKV> {
        let mut file = fs.open(&path).ok()?;
        let size = file.size();
        if size > 65_536 {
            // Sanity limit — nvs blobs should never be this large.
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        file.read_exact(&mut buf).ok()?;
        let text = core::str::from_utf8(&buf).ok()?;
        TinyKV::from_data(text).ok()
    })()
    .unwrap_or_else(TinyKV::new)
}

fn save_store(
    fs: &mut dyn FileSystem,
    ns: &str,
    store: &TinyKV,
) -> Result<(), PlatformError> {
    let path = nvs_path(ns);
    let data = store.to_data().map_err(|_| PlatformError::NvsError)?;
    fs.write_file(&path, data.as_bytes())
        .map_err(|_| PlatformError::NvsError)
}

// ─── Public API ──────────────────────────────────────────────────────────────

pub fn nvs_read(
    fs: &mut dyn FileSystem,
    ns: &str,
    key: &str,
) -> Result<Vec<u8>, PlatformError> {
    let mut store = load_store(fs, ns);
    let hex: Option<String> = store.get(key).map_err(|_| PlatformError::NvsError)?;
    let hex = hex.ok_or(PlatformError::NvsError)?;
    hex_decode(&hex).ok_or(PlatformError::NvsError)
}

pub fn nvs_write(
    fs: &mut dyn FileSystem,
    ns: &str,
    key: &str,
    val: &[u8],
) -> Result<(), PlatformError> {
    let mut store = load_store(fs, ns);
    store
        .set(key, hex_encode(val))
        .map_err(|_| PlatformError::NvsError)?;
    save_store(fs, ns, &store)
}

pub fn nvs_erase(
    fs: &mut dyn FileSystem,
    ns: &str,
    key: &str,
) -> Result<(), PlatformError> {
    let mut store = load_store(fs, ns);
    store.remove(key).map_err(|_| PlatformError::NvsError)?;
    save_store(fs, ns, &store)
}
