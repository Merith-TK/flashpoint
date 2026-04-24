/// FAT32 backend — wraps `embedded_sdmmc::VolumeManager` using the raw API.
///
/// The raw API (`RawVolume`, `RawDirectory`, `RawFile`) consists of plain
/// integer wrappers (Copy) with no embedded lifetimes, so they can be stored
/// in a struct alongside the VolumeManager that owns the actual state.
/// All raw handles must be explicitly closed; Drop impls handle that here.

use common::{FsError, FsFile, FileSystem, Platform};
use embedded_sdmmc::{Mode, RawDirectory, RawFile, RawVolume, TimeSource, Timestamp, VolumeIdx, VolumeManager};

use super::block::PlatformBlockDevice;

// Minimal no-op TimeSource — we don't care about file timestamps.
struct NoopTime;
impl TimeSource for NoopTime {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp::from_fat(0, 0)
    }
}

type Mgr<'a> = VolumeManager<PlatformBlockDevice<'a>, NoopTime>;

// ─── Fat32Fs ─────────────────────────────────────────────────────────────────

pub struct Fat32Fs<'a> {
    mgr: Mgr<'a>,
    raw_vol: RawVolume,
    raw_root: RawDirectory,
}

impl<'a> Fat32Fs<'a> {
    pub fn mount(platform: &'a dyn Platform) -> Result<Self, FsError> {
        let dev = PlatformBlockDevice::new(platform);
        let mut mgr = VolumeManager::new(dev, NoopTime);

        let raw_vol = mgr
            .open_raw_volume(VolumeIdx(0))
            .map_err(|e| {
                log::error!("[fs/fat] open_raw_volume failed: {:?}", e);
                FsError::InvalidFilesystem
            })?;

        let raw_root = mgr
            .open_root_dir(raw_vol)
            .map_err(|e| {
                log::error!("[fs/fat] open_root_dir failed: {:?}", e);
                FsError::InvalidFilesystem
            })?;

        Ok(Self { mgr, raw_vol, raw_root })
    }
}

impl<'a> Drop for Fat32Fs<'a> {
    fn drop(&mut self) {
        let _ = self.mgr.close_dir(self.raw_root);
        let _ = self.mgr.close_volume(self.raw_vol);
    }
}

impl<'a> FileSystem for Fat32Fs<'a> {
    fn open<'b>(&'b mut self, name: &str) -> Result<Box<dyn FsFile + 'b>, FsError> {
        let raw_file = self
            .mgr
            .open_file_in_dir(self.raw_root, name, Mode::ReadOnly)
            .map_err(|_| FsError::NotFound)?;

        let size = self
            .mgr
            .file_length(raw_file)
            .map_err(|_| FsError::Io)? as u64;

        Ok(Box::new(Fat32File {
            mgr: &mut self.mgr,
            raw: raw_file,
            size,
        }))
    }

    fn exists(&mut self, name: &str) -> bool {
        self.mgr
            .find_directory_entry(self.raw_root, name)
            .is_ok()
    }

    fn write_file(&mut self, name: &str, data: &[u8]) -> Result<(), FsError> {
        let raw_file = self
            .mgr
            .open_file_in_dir(self.raw_root, name, Mode::ReadWriteCreateOrTruncate)
            .map_err(|_| FsError::Io)?;

        let result = self.mgr.write(raw_file, data).map_err(|_| FsError::Io);
        let _ = self.mgr.close_file(raw_file);
        result.map(|_| ())
    }
}

// ─── Fat32File ────────────────────────────────────────────────────────────────

struct Fat32File<'mgr, 'dev: 'mgr> {
    mgr: &'mgr mut Mgr<'dev>,
    raw: RawFile,
    size: u64,
}

impl<'mgr, 'dev: 'mgr> Drop for Fat32File<'mgr, 'dev> {
    fn drop(&mut self) {
        let _ = self.mgr.close_file(self.raw);
    }
}

impl<'mgr, 'dev: 'mgr> FsFile for Fat32File<'mgr, 'dev> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        if self.mgr.file_eof(self.raw).map_err(|_| FsError::Io)? {
            return Ok(0);
        }
        self.mgr.read(self.raw, buf).map_err(|_| FsError::Io)
    }

    fn seek(&mut self, offset: u64) -> Result<(), FsError> {
        let off32 = u32::try_from(offset).map_err(|_| FsError::EndOfFile)?;
        self.mgr
            .file_seek_from_start(self.raw, off32)
            .map_err(|_| FsError::EndOfFile)
    }

    fn size(&self) -> u64 {
        self.size
    }
}
