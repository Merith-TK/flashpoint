/// PlatformBlockDevice — adapts `&dyn Platform` to both:
///   - `embedded_sdmmc::BlockDevice`  (FAT32 backend)
///   - `exfat_slim::blocking::io::BlockDevice` (exFAT backend)
///
/// Both traits are identical in terms of what they need: read/write a 512-byte
/// sector by LBA index.  We implement both for the same wrapper type.

use common::{Platform, PlatformError};

// ─── Shared wrapper ───────────────────────────────────────────────────────────

pub struct PlatformBlockDevice<'a> {
    platform: &'a dyn Platform,
}

impl<'a> PlatformBlockDevice<'a> {
    pub fn new(platform: &'a dyn Platform) -> Self {
        Self { platform }
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
                .sd_read_sectors(start_block_idx.0 + i as u32, &mut block.contents)?;
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
                .sd_write_sectors(start_block_idx.0 + i as u32, &block.contents)?;
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
        self.platform.sd_read_sectors(lba, block)
    }

    fn write(
        &mut self,
        lba: u32,
        block: &exfat_slim::blocking::io::Block,
    ) -> Result<(), Self::Error> {
        self.platform.sd_write_sectors(lba, block)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(()) // writes go straight to the HAL; no internal buffer to flush
    }
}
