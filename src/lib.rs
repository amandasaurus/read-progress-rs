//! Monitor how much you have read from a `Read`.
use std::path::PathBuf;
use std::io::Read;
use std::fs::File;
use std::time::{Instant, Duration};

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
    
    /// When did this reader start reading
    /// `None` if it hasn't started
    pub fn read_start_time(&self) -> Option<Instant> {
        self.read_start_time
    }

    /// Estimated Time to Arrival, at this rate, what's the predicted end time
    /// `None` if it hasn't started yet
    pub fn eta(&self) -> Option<Duration> {
        self.read_start_time.map(|read_start_time| {
            let duration_since_start = Instant::now() - read_start_time;
            duration_since_start.div_f64(self.fraction()) - duration_since_start
        })
    }

    /// Estimated Time to Completion, at this rate, how long before it is complete
    /// `None` if it hasn't started yet
    pub fn etc(&self) -> Option<Instant> {
        self.read_start_time.map(|read_start_time| {
            let duration_since_start = Instant::now() - read_start_time;
            read_start_time + duration_since_start.div_f64(self.fraction())
        })
    }

    /// Total estimated duration this reader will run for.
    /// `None` if it hasn't started yet
    pub fn est_total_time(&self) -> Option<Duration> {
        self.read_start_time.map(|read_start_time| {
            let duration_since_start = Instant::now() - read_start_time;
            duration_since_start.div_f64(self.fraction())
        })
    }

    /// How many bytes per second are being read.
    /// `None` if it hasn't started
    pub fn bytes_per_sec(&self) -> Option<f64> {
        self.read_start_time.map(|read_start_time| {
            let since_start = Instant::now() - read_start_time;
            (self.total_read as f64)/since_start.as_secs_f64()
        })
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

/// Read from this, storing how many bytes were read
impl<R> Read for ReaderWithSize<R> where R: Read {
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
