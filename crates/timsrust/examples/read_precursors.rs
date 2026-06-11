//! List the precursor table of a timsTOF dataset.
//!
//! Usage:
//!     cargo run --release --example read_precursors -- /path/to/data.d

use timsrust::TimsTofPath;

const PREVIEW_LIMIT: usize = 10;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_path = std::env::args()
        .nth(1)
        .expect("usage: read_precursors <path-to-data>");

    let path = TimsTofPath::new(&raw_path)?;
    let precursor_reader = path.precursor_reader()?;

    println!(
        "{} precursors in {raw_path} (showing first {PREVIEW_LIMIT}):",
        precursor_reader.len(),
    );
    println!("  index    m/z         charge      RT (s)     IM (1/K0)");

    for index in 0..precursor_reader.len().min(PREVIEW_LIMIT) {
        let p = precursor_reader.get(index)?;
        let charge = p
            .charge()
            .map(|c| i8::from(c).to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "  {:>5}    {:<11} {:<11} {:<10} {}",
            p.index(),
            p.mz().to_string(),
            charge,
            p.rt().to_string(),
            p.im(),
        );
    }
    Ok(())
}
