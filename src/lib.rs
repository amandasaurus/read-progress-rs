//! Monitor how much you have read from a `Read`.
use std::path::PathBuf;
use std::io::Read;
use std::fs::File;

/// A wrapper for a `Read` that monitors how many bytes have been read, and how many are to go
pub struct ReaderWithSize<R: Read> {
    inner: R,

    total_size: usize,
    total_read: usize,
}

impl<R: Read> ReaderWithSize<R> {
    /// Create a ReaderWithSize from `inner` presuming the total number of bytes is `total_size`.
    pub fn new(total_size: usize, inner: R) -> Self {
        ReaderWithSize{ total_size, total_read: 0, inner }
    }

    /// The total number of bytes that have been read from this reader
    pub fn total_read(&self) -> usize {
        self.total_read
    }

    /// The assumed total number of bytes in this reader, created when this object was created.
    pub fn assummed_total_size(&self) -> usize {
        self.total_size
    }


    /// How far along this reader have we read? What fraction have we read? May be >1.0 if the
    /// initial provided assumed total size was wrong.
    pub fn fraction(&self) -> f64 {
        (self.total_read as f64)/(self.total_size as f64)
    }

    /// Consumer this, and return the inner `Read`.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// A reference to the inner `Read`.
    pub fn inner(&self) -> &R {
        &self.inner
    }

}

impl ReaderWithSize<File> {
    /// Given a path, create a `ReaderWithSize` based on that file size
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let path: PathBuf = path.into();

        let file = File::open(path)?;
        let size = file.metadata()?.len() as usize;

        Ok(Self::new(size, file))
    }
}

/// Read from this, storing how many bytes were read
impl<R> Read for ReaderWithSize<R> where R: Read {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let result = self.inner.read(buf);
        if let Ok(bytes_read) = result {
            self.total_read += bytes_read;
        }
        result
    }
}
