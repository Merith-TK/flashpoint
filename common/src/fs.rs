/// Filesystem abstraction — the only FS types visible to callers.
///
/// Backends (FAT32 via embedded-sdmmc, exFAT via exfat-slim) live in
/// `firmware/src/fs/` and are never imported above that layer.
/// Everything here is `no_std`-safe.

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// File or directory not found.
    NotFound,
    /// Low-level block I/O error.
    Io,
    /// Volume boot record not recognised as FAT32 or exFAT.
    InvalidFilesystem,
    /// Read/seek attempted past end of file.
    EndOfFile,
    /// Filesystem operation not supported on this volume type.
    Unsupported,
}

// ─── File ────────────────────────────────────────────────────────────────────

/// An open, readable (and optionally seekable) file.
///
/// Backends return a concrete type that implements this trait wrapped in a
/// `Box<dyn FsFile>`.  Callers see nothing else.
pub trait FsFile {
    /// Read bytes into `buf`, returning how many were filled.
    /// Returns `Ok(0)` at end-of-file; returns `Err(FsError::EndOfFile)` only
    /// if called again after `Ok(0)`.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError>;

    /// Read exactly `buf.len()` bytes, returning `Err(EndOfFile)` if the file
    /// is shorter.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), FsError> {
        let mut pos = 0;
        while pos < buf.len() {
            let n = self.read(&mut buf[pos..])?;
            if n == 0 {
                return Err(FsError::EndOfFile);
            }
            pos += n;
        }
        Ok(())
    }

    /// Seek to an absolute byte offset from the start of the file.
    fn seek(&mut self, offset: u64) -> Result<(), FsError>;

    /// Total size of the file in bytes.
    fn size(&self) -> u64;
}

// ─── Filesystem ──────────────────────────────────────────────────────────────

/// A mounted, read-capable filesystem volume backed by raw SD sectors.
pub trait FileSystem {
    /// Open a file by name in the root directory.
    fn open<'a>(&'a mut self, name: &str) -> Result<Box<dyn FsFile + 'a>, FsError>;

    /// Check whether a file exists in the root directory without opening it.
    fn exists(&mut self, name: &str) -> bool;

    /// Write (create or overwrite) a file in the root directory with `data`.
    ///
    /// Default implementation returns `Err(FsError::Unsupported)`.  Backends
    /// that support writes must override this.
    fn write_file(&mut self, _name: &str, _data: &[u8]) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }
}
