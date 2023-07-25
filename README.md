# read-progress

This `std::io::Read` wrapper allows you to answer: “How much of this file has been read?”

Monitor how much you have read from a `Read`.

# Usage

```rust,ignore
use read_progress::ReaderWithSize;
let mut rdr = ReaderWithSize::from_file(file)?;
// ...
// ... [ perform regular reads ]
rdr.fraction()         // 0 (nothing) → 1 (everything) with how much of the file has been read

// Based on how fast the file is being read you can call:
rdr.eta()              // `std::time::Duration` with how long until it's finished
rdr.est_total_time()   // `std::time::Instant` when, at this rate, it'll be finished
```

## Copyright

Available under the MIT, or Apache-2.0 or GNU Affero GPL 3.0+.
