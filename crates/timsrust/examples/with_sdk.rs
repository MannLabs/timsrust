//! Use the Bruker SDK calibration backend.
//!
//! Build with:
//!     cargo run --release --example with_sdk --features sdk -- /path/to/data.d
//!
//! Requires the Bruker `libtimsdata` shared library to be on the dynamic
//! linker search path (e.g. via `LD_LIBRARY_PATH`); see the README for
//! details.

use timsrust::{MzConverter, TimsTofPath};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_path = std::env::args()
        .nth(1)
        .expect("usage: with_sdk <path-to-tdf-data>");

    let path = TimsTofPath::new(&raw_path)?;
    let converter = path.mz_converter().expect("no m/z calibration available");

    match converter {
        MzConverter::Sdk(_) => {
            println!("Using Bruker SDK calibration for {raw_path}")
        },
        other => println!(
            "SDK feature is enabled but the path is not a TDF folder \
             (got {other:?}); falling back to a non-SDK converter.",
        ),
    }

    let reader = path.spectrum_reader()?;
    println!("Loaded {} spectra.", reader.len());
    Ok(())
}
