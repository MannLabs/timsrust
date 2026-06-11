//! Read all centroided spectra from a timsTOF dataset.
//!
//! Usage:
//!     cargo run --release --example read_spectra -- /path/to/data.d
//!
//! Works for any format auto-detected by [`TimsTofPath`] (TDF, miniTDF,
//! Parquet).

use timsrust::TimsTofPath;

const PREVIEW_LIMIT: usize = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_path = std::env::args()
        .nth(1)
        .expect("usage: read_spectra <path-to-data>");

    let path = TimsTofPath::new(&raw_path)?;
    let spectrum_reader = path.spectrum_reader()?;

    println!("Loaded {} spectra from {raw_path}", spectrum_reader.len());

    for index in 0..spectrum_reader.len().min(PREVIEW_LIMIT) {
        let spectrum = spectrum_reader.get(index)?;
        let precursor_mz = spectrum
            .precursor()
            .as_ref()
            .map(|p| p.mz().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        println!(
            "  [{:>4}] precursor m/z = {precursor_mz}, {} peaks",
            spectrum.index(),
            spectrum.len(),
        );
    }
    Ok(())
}
