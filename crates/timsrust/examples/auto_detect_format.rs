//! Demonstrate that the same code works for any supported timsTOF format.
//!
//! Usage:
//!     cargo run --release --example auto_detect_format -- /path/to/data
//!
//! Accepts `.d` (TDF), miniTDF folders, and Parquet spectra paths. Frame-level
//! access is only available for TDF data; the example reports whether the
//! input supports it.

use timsrust::TimsTofPath;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_path = std::env::args()
        .nth(1)
        .expect("usage: auto_detect_format <path-to-data>");

    let path = TimsTofPath::new(&raw_path)?;

    let spectrum_reader = path.spectrum_reader()?;
    let precursor_reader = path.precursor_reader()?;
    let frame_supported = path.frame_reader().is_ok();
    let mz_supported = path.mz_converter().is_some();

    println!("Path:              {raw_path}");
    println!("Spectra:           {}", spectrum_reader.len());
    println!("Precursors:        {}", precursor_reader.len());
    println!("Frame access:      {frame_supported}");
    println!("M/Z calibration:   {mz_supported}");
    Ok(())
}
