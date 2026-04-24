/// PlatformBlockDevice — adapts `&dyn Platform` to both:
///   - `embedded_sdmmc::BlockDevice`  (FAT32 backend)
///   - `exfat_slim::blocking::io::BlockDevice` (exFAT backend)
///
/// `lba_offset` is added to every sector number before the I/O call.  Use 0
/// for whole-disk formats and the partition's LBA start for MBR-partitioned
/// cards — exfat-slim requires that LBA 0 maps to the exFAT boot sector.

use common::{Platform, PlatformError};

// ─── Shared wrapper ────────────────────────────────────────────────────────────

pub struct PlatformBlockDevice<'a> {
    platform: &'a dyn Platform,
    /// Added to every LBA before passing to the platform.
    lba_offset: u32,
}

impl core::fmt::Debug for PlatformBlockDevice<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PlatformBlockDevice {{ lba_offset: {} }}", self.lba_offset)
    }
}

impl<'a> PlatformBlockDevice<'a> {
    pub fn new(platform: &'a dyn Platform) -> Self {
        Self { platform, lba_offset: 0 }
    }

    pub fn with_offset(platform: &'a dyn Platform, lba_offset: u32) -> Self {
        Self { platform, lba_offset }
    }
}

// ─── embedded-sdmmc BlockDevice ──────────────────────────────────────────────

#[cfg(feature = "embedded-sdmmc")]
impl<'a> embedded_sdmmc::BlockDevice for PlatformBlockDevice<'a> {
    type Error = PlatformError;

    fn read(
        &self,
        blocks: &mut [embedded_sdmmc::Block],
        start_block_idx: embedded_sdmmc::BlockIdx,
        _reason: &str,
    ) -> Result<(), Self::Error> {
        for (i, block) in blocks.iter_mut().enumerate() {
            self.platform
                .sd_read_sectors(self.lba_offset + start_block_idx.0 + i as u32, &mut block.contents)?;
        }
        Ok(())
    }

    fn write(
        &self,
        blocks: &[embedded_sdmmc::Block],
        start_block_idx: embedded_sdmmc::BlockIdx,
    ) -> Result<(), Self::Error> {
        for (i, block) in blocks.iter().enumerate() {
            self.platform
                .sd_write_sectors(self.lba_offset + start_block_idx.0 + i as u32, &block.contents)?;
        }
        Ok(())
    }

    fn num_blocks(&self) -> Result<embedded_sdmmc::BlockCount, Self::Error> {
        Ok(embedded_sdmmc::BlockCount(self.platform.sd_sector_count()))
    }
}

// ─── exfat-slim BlockDevice ───────────────────────────────────────────────────

#[cfg(feature = "exfat-slim")]
impl<'a> exfat_slim::blocking::io::BlockDevice for PlatformBlockDevice<'a> {
    type Error = PlatformError;

    fn read(
        &mut self,
        lba: u32,
        block: &mut exfat_slim::blocking::io::Block,
    ) -> Result<(), Self::Error> {
        self.platform.sd_read_sectors(self.lba_offset + lba, block)
    }

    fn write(
        &mut self,
        lba: u32,
        block: &exfat_slim::blocking::io::Block,
    ) -> Result<(), Self::Error> {
        self.platform.sd_write_sectors(self.lba_offset + lba, block)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(()) // writes go straight to the HAL; no internal buffer to flush
    }
}
