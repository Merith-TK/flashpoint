/// exFAT backend — wraps `exfat_slim::blocking::file_system::FileSystem`.
///
/// Design note: exfat-slim's `File::read` / `File::seek` both require
/// `&mut FileSystem<D>` as a parameter, so `ExFatFile` owns the filesystem
/// via an `Option` taken from `ExFatFs` at open time.  Only one file may be
/// open at a time, which is fine for our ROM-loading use-case.

extern crate alloc;
use alloc::string::String;

use common::{FsError, FsFile, FileSystem as FpFileSystem, Platform};
use exfat_slim::blocking::{
    file::OpenOptions,
    file_system::FileSystem as ExFileSystem,
};

use super::block::PlatformBlockDevice;

type ExFs<'a> = ExFileSystem<PlatformBlockDevice<'a>>;

// ─── ExFatFs ─────────────────────────────────────────────────────────────────

pub struct ExFatFs<'a> {
    // Held as Option so we can move it into ExFatFile when opening.
    inner: Option<ExFs<'a>>,
}

impl<'a> ExFatFs<'a> {
    pub fn mount(platform: &'a dyn Platform) -> Result<Self, FsError> {
        let dev = PlatformBlockDevice::new(platform);
        let mut fs = ExFs::new(dev);
        fs.mount().map_err(|_| FsError::InvalidFilesystem)?;
        Ok(Self { inner: Some(fs) })
    }

    fn to_path(name: &str) -> String {
        if name.starts_with('/') {
            String::from(name)
        } else {
            alloc::format!("/{name}")
        }
    }
}

impl<'a> FpFileSystem for ExFatFs<'a> {
    // 'b is the borrow of self; since 'a: 'b (platform outlives borrow),
    // ExFatFile<'a>: 'b holds and satisfies Box<dyn FsFile + 'b>.
    fn open<'b>(&'b mut self, name: &str) -> Result<Box<dyn FsFile + 'b>, FsError> {
        let path = Self::to_path(name);
        let opts = OpenOptions::new().read(true);
        // Take the FS out of the Option (returns None check below).
        let mut fs = self.inner.take().ok_or(FsError::Unsupported)?;
        let file = match fs.open(&path, opts) {
            Ok(f) => f,
            Err(_) => {
                // Restore on error so ExFatFs remains usable.
                self.inner = Some(fs);
                return Err(FsError::NotFound);
            }
        };
        let size = file.metadata().len();
        Ok(Box::new(ExFatFile { fs, file, size }))
    }

    fn exists(&mut self, name: &str) -> bool {
        let path = Self::to_path(name);
        if let Some(fs) = self.inner.as_mut() {
            fs.exists(&path).unwrap_or(false)
        } else {
            false
        }
    }
}

// ─── ExFatFile ────────────────────────────────────────────────────────────────

struct ExFatFile<'a> {
    fs: ExFs<'a>,
    file: exfat_slim::blocking::file::File,
    size: u64,
}

impl<'a> FsFile for ExFatFile<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        match self.file.read(&mut self.fs, buf).map_err(|_| FsError::Io)? {
            Some(n) => Ok(n),
            None => Ok(0), // EOF
        }
    }

    fn seek(&mut self, offset: u64) -> Result<(), FsError> {
        self.file
            .seek(&mut self.fs, offset)
            .map_err(|_| FsError::EndOfFile)
    }

    fn size(&self) -> u64 {
        self.size
    }
}
