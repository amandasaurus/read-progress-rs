//! This `std::io::Read` wrapper allows you to answer: “How much of this file has been read?”
//!
//! Monitor how much you have read from a `Read`.
//!
//! # Usage
//!
//! ```rust
//! use read_progress::ReaderWithSize;
//! let mut rdr = ReaderWithSize::from_file(file)?;
//! ...
//! ... [ perform regular reads ]
//! rdr.fraction()         // 0 (nothing) → 1 (everything) with how much of the file has been read
//!
//! // Based on how fast the file is being read you can call:
//! rdr.eta()              // `std::time::Duration` with how long until it's finished
//! rdr.est_total_time()   // `std::time::Instant` when, at this rate, it'll be finished
//! ```
use std::path::PathBuf;
use std::io::Read;
use std::io::BufReader;
use std::fs::File;
use std::time::{Instant, Duration};

pub trait ReadWithSize: Read {
    ///// The read function
    //fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    /// The total number of bytes that have been read from this reader
    fn total_read(&self) -> usize;

    /// The assumed total number of bytes in this reader, created when this object was created.
    fn assummed_total_size(&self) -> usize;

    /// How far along this reader have we read? What fraction have we read? May be >1.0 if the
    /// initial provided assumed total size was wrong.
    fn fraction(&self) -> f64;

    /// When did this reader start reading
    /// `None` if it hasn't started
    fn read_start_time(&self) -> Option<Instant>;

    /// Estimated Time to Arrival, at this rate, what's the predicted end time
    /// `None` if it hasn't started yet
    fn eta(&self) -> Option<Duration> {
        self.read_start_time().map(|read_start_time| {
            let duration_since_start = Instant::now() - read_start_time;
            duration_since_start.div_f64(self.fraction()) - duration_since_start
        })
    }

    /// Estimated Time to Completion, at this rate, how long before it is complete
    /// `None` if it hasn't started yet
    fn etc(&self) -> Option<Instant> {
        self.read_start_time().map(|read_start_time| {
            let duration_since_start = Instant::now() - read_start_time;
            read_start_time + duration_since_start.div_f64(self.fraction())
        })
    }

    /// Total estimated duration this reader will run for.
    /// `None` if it hasn't started yet
    fn est_total_time(&self) -> Option<Duration> {
        self.read_start_time().map(|read_start_time| {
            let duration_since_start = Instant::now() - read_start_time;
            duration_since_start.div_f64(self.fraction())
        })
    }

    /// How many bytes per second are being read.
    /// `None` if it hasn't started
    fn bytes_per_sec(&self) -> Option<f64> {
        self.read_start_time().map(|read_start_time| {
            let since_start = Instant::now() - read_start_time;
            (self.total_read() as f64)/since_start.as_secs_f64()
        })
    }


}

/// A wrapper for a `Read` that monitors how many bytes have been read, and how many are to go
pub struct ReaderWithSize<R: Read> {
    inner: R,

    total_size: usize,
    total_read: usize,
    read_start_time: Option<Instant>,
}

impl<R: Read> ReaderWithSize<R> {
    /// Create a ReaderWithSize from `inner` presuming the total number of bytes is `total_size`.
    pub fn new(total_size: usize, inner: R) -> Self {
        ReaderWithSize{ total_size, total_read: 0, inner, read_start_time: None }
    }

    /// Consumer this, and return the inner `Read`.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// A reference to the inner `Read`.
    pub fn inner(&self) -> &R {
        &self.inner
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let result = self.inner.read(buf);
        if let Ok(bytes_read) = result {
            self.total_read += bytes_read;
        }
        if self.read_start_time.is_none() {
            self.read_start_time = Some(Instant::now());
        }
        result
    }

}

impl<R: Read> ReadWithSize for ReaderWithSize<R> {

    /// The total number of bytes that have been read from this reader
    fn total_read(&self) -> usize {
        self.total_read
    }

    /// The assumed total number of bytes in this reader, created when this object was created.
    fn assummed_total_size(&self) -> usize {
        self.total_size
    }

    /// How far along this reader have we read? What fraction have we read? May be >1.0 if the
    /// initial provided assumed total size was wrong.
    fn fraction(&self) -> f64 {
        (self.total_read as f64)/(self.total_size as f64)
    }

    /// When did this reader start reading
    /// `None` if it hasn't started
    fn read_start_time(&self) -> Option<Instant> {
        self.read_start_time
    }

}

impl<R: Read> Read for ReaderWithSize<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read(buf)
    }
}

impl ReaderWithSize<File> {
    /// Given a path, create a `ReaderWithSize` based on that file size
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let path: PathBuf = path.into();

        let file = File::open(path)?;
        ReaderWithSize::from_file(file)
    }

    /// Given a file, create a `ReaderWithSize` based on that file size
    pub fn from_file(file: File) -> Result<Self, std::io::Error> {
        let size = file.metadata()?.len() as usize;

        Ok(Self::new(size, file))
    }
}


pub struct BufReaderWithSize<R: Read>(BufReader<ReaderWithSize<R>>);

impl BufReaderWithSize<File> {
    /// Given a path, create a `BufReaderWithSize` based on that file size
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let path: PathBuf = path.into();

        let file = File::open(path)?;

        BufReaderWithSize::from_file(file)
    }

    /// Given a file, create a `BufReaderWithSize` based on that file size
    pub fn from_file(file: File) -> Result<Self, std::io::Error> {
        let size = file.metadata()?.len() as usize;

        let rdr = ReaderWithSize::new(size, file);
        let rdr = BufReader::new(rdr);

        Ok(BufReaderWithSize(rdr))
    }
}


impl<R: Read> Read for BufReaderWithSize<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::thread::sleep;

    #[test]
    fn basic() {
        let bytes = "hello".as_bytes();
        let mut reader = ReaderWithSize::new(5, Cursor::new(bytes));
        assert_eq!(reader.assummed_total_size(), 5);

        let mut buf = vec![0];

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, vec!['h' as u8]);
        assert_eq!(reader.total_read(), 1);
        assert_eq!(reader.fraction(), 0.2);

        let mut buf = vec![0, 0];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, vec!['e' as u8, 'l' as u8]);
        assert_eq!(reader.total_read(), 3);
        assert_eq!(reader.fraction(), 0.6);

        let _cursor: &Cursor<&[u8]> = reader.inner();
        let _cursor: Cursor<&[u8]> = reader.into_inner();
    }

    #[test]
    fn eta1() {
        let start = Instant::now();
        let bytes = "hello".as_bytes();
        let mut reader = ReaderWithSize::new(5, Cursor::new(bytes));

        // haven't started running yet
        assert_eq!(reader.eta(), None);

        let mut buf = vec![0];
        reader.read_exact(&mut buf).unwrap();

        // wait 10ms
        sleep(Duration::from_millis(10));

        // The ETA won't be exactly 40ms, becase code takes a little bit to run. Confirm that it's
        // between 40 & 41 ms.
        let eta = reader.eta();
        let bytes_per_sec = reader.bytes_per_sec();
        let etc = reader.etc();

        assert!(eta.is_some());
        let eta: Duration = eta.unwrap();

        assert!(eta >= Duration::from_millis(40));
        assert!(40./1000. - eta.as_secs_f64() <= 1.);


        assert!(bytes_per_sec.is_some());
        let bytes_per_sec: f64 = bytes_per_sec.unwrap();
        assert!(bytes_per_sec >=  20.);   // ≥ 1 byte per 50ms
        assert!(bytes_per_sec < 100.);

        assert!(etc.is_some());
        let etc: Instant = etc.unwrap();
        assert!(etc > start);
        assert!(etc < start+Duration::from_secs(1));
    }

}
