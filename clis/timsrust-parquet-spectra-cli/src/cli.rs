use std::path::PathBuf;

use clap::{Parser, ValueHint};

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
        long = "min-spectrum-size",
        short = 's',
        default_value_t = 5,
        help = "Minimum number of peaks required for a spectrum to be written (only for .mgf output)"
    )]
    min_spectrum_size: usize,
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
    pub fn run() {
        let input = Input::parse();
        run(input.in_path, input.out_path, input.min_spectrum_size);
    }
}
