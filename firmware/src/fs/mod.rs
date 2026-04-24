/// Filesystem abstraction for the firmware.
///
/// `mount()` reads the first sector of the SD card and dispatches to the
/// correct backend:
///   - exFAT  → `exfat.rs`  (exfat-slim, for large SDXC cards)
///   - FAT32  → `fat.rs`    (embedded-sdmmc, for smaller SD/SDHC cards)
///
/// Detection strategy:
///   1. Read sector 0.  If bytes 3..11 == "EXFAT   " the card is whole-disk
///      exFAT (no MBR).  Mount exFAT.
///   2. Otherwise assume sector 0 is an MBR.  Parse partition entry 0 to get
///      the LBA start of the first partition and its type byte:
///        0x07 → exFAT (Windows SDXC default) → mount exFAT at partition LBA
///        0x0B / 0x0C → FAT32 → mount FAT32
///      If parsing fails or the type is unrecognised, fall back to FAT32 on
///      the assumption the card has no partition table ("superfloppy" format).
///
/// Callers only ever see `Box<dyn common::FileSystem>`.

#[cfg(feature = "embedded-sdmmc")]
pub mod fat;

#[cfg(feature = "exfat-slim")]
pub mod exfat;

pub(crate) mod block;

use common::{FileSystem, FsError, Platform};

// MBR partition type bytes
const PART_EXFAT: u8 = 0x07;
const PART_FAT32_CHS: u8 = 0x0B;
const PART_FAT32_LBA: u8 = 0x0C;

/// Mount the SD card's first partition as either exFAT or FAT32.
pub fn mount<'a>(platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    let mut sector = [0u8; 512];
    platform
        .sd_read_sectors(0, &mut sector)
        .map_err(|_| FsError::Io)?;

    // Whole-disk exFAT: OEM name at offset 3 is "EXFAT   ".
    if &sector[3..11] == b"EXFAT   " {
        log::info!("[fs] detected whole-disk exFAT");
        return mount_exfat(platform);
    }

    // Try MBR partition table.  The MBR signature 0xAA55 lives at offset 510.
    // Partition entry 0 starts at offset 446; its type byte is at offset 448
    // (entry relative: 2), LBA start at offset 454 (entry relative: 8).
    let mbr_sig = u16::from_le_bytes([sector[510], sector[511]]);
    if mbr_sig == 0xAA55 {
        let entry = &sector[446..462]; // 16-byte partition entry 0
        let part_type = entry[4];
        let lba_start = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
        log::info!("[fs] MBR found — partition type 0x{:02X}, LBA start {}", part_type, lba_start);

        match part_type {
            PART_EXFAT => {
                log::info!("[fs] partition type 0x07 → exFAT (LBA offset {})", lba_start);
                return mount_exfat_at(platform, lba_start);
            }
            PART_FAT32_CHS | PART_FAT32_LBA => {
                log::info!("[fs] partition type 0x{:02X} → FAT32", part_type);
                return mount_fat(platform);
            }
            _ => {
                log::warn!("[fs] unknown partition type 0x{:02X}, trying FAT32", part_type);
            }
        }
    } else {
        log::info!("[fs] no MBR signature (0x{:04X}), trying FAT32 as whole-disk", mbr_sig);
    }

    mount_fat(platform)
}

#[cfg(feature = "exfat-slim")]
fn mount_exfat<'a>(platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    exfat::ExFatFs::mount(platform, 0).map(|fs| Box::new(fs) as Box<dyn FileSystem>)
}

#[cfg(feature = "exfat-slim")]
fn mount_exfat_at<'a>(platform: &'a dyn Platform, lba_offset: u32) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    exfat::ExFatFs::mount(platform, lba_offset).map(|fs| Box::new(fs) as Box<dyn FileSystem>)
}

#[cfg(not(feature = "exfat-slim"))]
fn mount_exfat<'a>(_platform: &'a dyn Platform) -> Result<Box<dyn FileSystem + 'a>, FsError> {
    Err(FsError::Unsupported)
}

#[cfg(not(feature = "exfat-slim"))]
fn mount_exfat_at<'a>(_platform: &'a dyn Platform, _lba_offset: u32) -> Result<Box<dyn FileSystem + 'a>, FsError> {
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
