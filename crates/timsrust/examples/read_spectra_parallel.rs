//! Iterate spectra in parallel using rayon.
//!
//! Usage:
//!     cargo run --release --example read_spectra_parallel -- /path/to/data.d
//!
//! Demonstrates [`SpectrumReader::par_iter`], which yields
//! `Result<Spectrum<Mz>, _>` items in parallel.

use rayon::prelude::*;
use timsrust::{SpectrumReader, TimsTofPath};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_path = std::env::args()
        .nth(1)
        .expect("usage: read_spectra_parallel <path-to-data>");

    let path = TimsTofPath::new(&raw_path)?;
    let reader = SpectrumReader::build().with_path(&path).finalize()?;

    let total_peaks: usize = reader
        .par_iter()
        .filter_map(|spectrum| spectrum.ok())
        .map(|spectrum| spectrum.len())
        .sum();

    println!("Total peaks across {} spectra: {total_peaks}", reader.len(),);
    Ok(())
}
