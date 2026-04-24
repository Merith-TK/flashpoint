/// Filesystem abstraction for the firmware.
///
/// `mount()` reads the first sector of the SD card and dispatches to the
/// correct backend:
///   - exFAT  → `exfat.rs`  (exfat-slim, for large SDXC cards)
///   - FAT32  → `fat.rs`    (embedded-sdmmc, for smaller SD/SDHC cards)
///
/// Callers only ever see `Box<dyn common::FileSystem>`.

#[cfg(feature = "embedded-sdmmc")]
pub mod fat;

#[cfg(feature = "exfat-slim")]
pub mod exfat;

pub(crate) mod block;

use common::{FileSystem, FsError, Platform};

/// Mount the SD card's first partition as either exFAT or FAT32.
///
/// Detection: the exFAT boot sector stores the ASCII string `"EXFAT   "` at
/// byte offset 3 of sector 0.
pub fn mount<'a>(platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    // Read the first sector to detect the filesystem type.
    let mut sector = [0u8; 512];
    platform
        .sd_read_sectors(0, &mut sector)
        .map_err(|_| FsError::Io)?;

    // exFAT signature at offset 3, length 8.
    let is_exfat = &sector[3..11] == b"EXFAT   ";

    mount_detected(platform, is_exfat)
}

/// Internal dispatcher — separated so cfg blocks produce correct return types.
fn mount_detected<'a>(
    platform: &'a dyn Platform,
    is_exfat: bool,
) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    if is_exfat {
        return mount_exfat(platform);
    }
    mount_fat(platform)
}

#[cfg(feature = "exfat-slim")]
fn mount_exfat<'a>(platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    let fs = exfat::ExFatFs::mount(platform)?;
    Ok(Box::new(fs))
}

#[cfg(not(feature = "exfat-slim"))]
fn mount_exfat<'a>(_platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    Err(FsError::Unsupported)
}

#[cfg(feature = "embedded-sdmmc")]
fn mount_fat<'a>(platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    let fs = fat::Fat32Fs::mount(platform)?;
    Ok(Box::new(fs))
}

#[cfg(not(feature = "embedded-sdmmc"))]
fn mount_fat<'a>(_platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    Err(FsError::Unsupported)
}
