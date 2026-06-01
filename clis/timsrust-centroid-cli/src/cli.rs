use std::path::PathBuf;

use clap::{Parser, ValueHint};
use timsrust_centroid::TimsResult;

use crate::run;

#[derive(Parser, Debug)]
#[command(name = "timscentroid", version = clap::crate_version!(), author = "BrukerProteoScape", about = "Centroid TimsTof data in 2D")]
struct Input {
    #[arg(
        help = "Path to TimsTof data (i.e. (any file within) a .d folder)",
        value_hint = ValueHint::FilePath
    )]
    in_path: String,
    #[arg(
        long = "out-path",
        short = 'o',
        default_value = "./peaks.parquet",
        help = "Path to a results file (WARNING: overwrites existing files). Supported formats: .parquet, .mgf, .spec.parquet",
        value_hint = ValueHint::FilePath,
        value_parser = validate_output_path,
    )]
    out_path: String,
    #[arg(
        long = "min-ion-count_ms1",
        short = 'm',
        default_value_t = 0.5,
        help = "Minimum ion count (detector events, not intensity) to filter noise centroids in MS1. If <1.0, interpreted as fraction of detected scan FWHM."
    )]
    min_ion_count_ms1: f64,
    #[arg(
        long = "min-ion-count_ms2",
        short = 'n',
        default_value_t = 2.0,
        help = "Minimum ion count (detector events, not intensity) to filter noise centroids in MS2. If <1.0, interpreted as fraction of detected scan FWHM."
    )]
    min_ion_count_ms2: f64,
    #[arg(
        long = "min-spectrum-size",
        short = 's',
        default_value_t = 5,
        help = "Minimum number of peaks required for a spectrum to be written (only for .mgf output)"
    )]
    min_spectrum_size: usize,
    #[arg(
        long = "ignore-precursors",
        short = 'p',
        default_value_t = false,
        help = "Whether to ignore precursor information for mgf generation"
    )]
    ignore_precursors: bool,
}

fn validate_output_path(s: &str) -> Result<String, String> {
    let path = PathBuf::from(s);
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.to_lowercase())
    {
        Some(ref ext) if ext == "parquet" || ext == "mgf" => {
            Ok(path.to_string_lossy().to_string())
        },
        _ => Err(String::from(
            "Invalid file extension. Must be .parquet or .mgf",
        )),
    }
}

/// Command-line interface entry point for timsrust_centroid.
pub struct CLI {}

impl CLI {
    /// Run the CLI application.
    ///
    /// Parses arguments and invokes the main runner.
    ///
    /// # Errors
    /// Returns a `TimsError` if the runner fails.
    pub fn run() -> TimsResult<()> {
        let input = Input::parse();
        run(
            input.in_path,
            input.out_path,
            input.min_ion_count_ms1,
            input.min_ion_count_ms2,
            input.min_spectrum_size,
            !input.ignore_precursors,
        )?;
        Ok(())
    }
}
