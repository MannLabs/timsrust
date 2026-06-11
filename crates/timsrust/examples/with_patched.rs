//! Use a patched calibration backend (`timsrust-patched`).
//!
//! Build with:
//!     cargo run --release --example with_patched --features patched -- /path/to/data.d
//!
//! By default, this uses the published `timsrust-patched` crate. To swap in
//! your own implementation, add a `[patch.crates-io]` entry to your
//! `Cargo.toml`:
//!
//! ```toml
//! [patch.crates-io]
//! timsrust-patched = { path = "../my-timsrust-patched" }
//! ```

use timsrust::{MzConverter, TimsTofPath};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_path = std::env::args()
        .nth(1)
        .expect("usage: with_patched <path-to-tdf-data>");

    let path = TimsTofPath::new(&raw_path)?;
    let converter = path.mz_converter().expect("no m/z calibration available");

    match converter {
        MzConverter::Bps(_) => {
            println!("Using `timsrust-patched` calibration for {raw_path}",)
        },
        other => println!(
            "patched feature is enabled but the path is not a TDF folder \
             (got {other:?}); falling back to a non-patched converter.",
        ),
    }

    let reader = path.spectrum_reader()?;
    println!("Loaded {} spectra.", reader.len());
    Ok(())
}
