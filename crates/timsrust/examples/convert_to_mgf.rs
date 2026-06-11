//! Convert spectra to a Mascot Generic Format (MGF) file.
//!
//! Usage:
//!     cargo run --release --example convert_to_mgf -- /path/to/data.d output.mgf
//!
//! Demonstrates pairing [`timsrust::SpectrumReader`] with
//! [`timsrust_mgf::MGFWriter`].

use rayon::prelude::*;
use std::sync::Mutex;

use timsrust::{SpectrumReader, TimsTofPath};
use timsrust_mgf::MGFWriter;

const MIN_PEAKS: usize = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let raw_path = args
        .next()
        .expect("usage: convert_to_mgf <input> <output.mgf>");
    let out_path = args
        .next()
        .expect("usage: convert_to_mgf <input> <output.mgf>");

    let path = TimsTofPath::new(&raw_path)?;
    let reader = SpectrumReader::build().with_path(&path).finalize()?;

    let writer = Mutex::new(MGFWriter::new(&out_path));
    let written: usize = reader
        .par_iter()
        .filter_map(|spectrum| spectrum.ok())
        .filter(|spectrum| spectrum.len() >= MIN_PEAKS)
        .filter(|spectrum| spectrum.precursor().is_some())
        .map(|spectrum| {
            writer
                .lock()
                .expect("MGFWriter mutex poisoned")
                .write(&spectrum);
            1usize
        })
        .sum();

    println!("Wrote {written} spectra to {out_path}");
    Ok(())
}
