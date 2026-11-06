use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum, ValueHint};
use timsrust_centroid::TimsResult;

use crate::run;
use timsrust_centroid::neighborhood::{
    AcquisitionPolicy, Config, ContextExtent, RtConsistencyConfig,
};

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Sampling {
    #[default]
    Window,
    Precursor,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Acquisition {
    #[default]
    SingleInterval,
    SharedTransmission,
    ExperimentalTrajectory,
}

impl From<Acquisition> for AcquisitionPolicy {
    fn from(value: Acquisition) -> Self {
        match value {
            Acquisition::SingleInterval => Self::SingleInterval,
            Acquisition::SharedTransmission => Self::SharedTransmission,
            Acquisition::ExperimentalTrajectory => Self::ExperimentalTrajectory,
        }
    }
}

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
        help = "Legacy output file (.parquet, .mgf, .spec.parquet; overwrites), or a new output directory with --neighborhoods",
        value_hint = ValueHint::FilePath,
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
        conflicts_with = "neighborhoods",
        short = 's',
        default_value_t = 5,
        help = "Minimum number of peaks required for a spectrum to be written (only for .mgf output)"
    )]
    min_spectrum_size: usize,
    #[arg(
        long = "ignore-precursors",
        conflicts_with = "neighborhoods",
        short = 'p',
        default_value_t = false,
        help = "Whether to ignore precursor information for mgf generation"
    )]
    ignore_precursors: bool,
    /// Export a fresh directory of versioned neighborhood Parquet tables.
    #[arg(long)]
    neighborhoods: bool,
    #[arg(long, value_enum, default_value_t = Sampling::Window, requires = "neighborhoods")]
    sampling: Sampling,
    /// Acquisition compatibility policy; trajectory mode is experimental.
    #[arg(long, value_enum, default_value_t = Acquisition::SingleInterval, requires = "neighborhoods")]
    acquisition: Acquisition,
    /// Enable or disable RT-supported centroid admission explicitly.
    #[arg(long, action=ArgAction::Set, num_args=0..=1, default_missing_value="true",
          default_value_t=false, requires="neighborhoods")]
    rt_centroiding: bool,
    #[arg(long, default_value_t = 2, requires = "neighborhoods")]
    rt_before: usize,
    #[arg(long, default_value_t = 2, requires = "neighborhoods")]
    rt_after: usize,
    #[arg(long, default_value_t = 0, requires = "neighborhoods")]
    im_neighbors: usize,
    /// Zero exports uncapped observations.
    #[arg(long, default_value_t = 0, requires = "neighborhoods")]
    peak_cap: usize,
    /// Export a bounded prefix for testing; recorded in the manifest.
    #[arg(long, requires = "neighborhoods")]
    max_bundles: Option<std::num::NonZeroUsize>,
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
        if input.neighborhoods {
            let config = Config {
                acquisition: input.acquisition.into(),
                rt_context: ContextExtent {
                    before: input.rt_before,
                    after: input.rt_after,
                },
                im_context: ContextExtent {
                    before: input.im_neighbors,
                    after: input.im_neighbors,
                },
                rt_consistency: input
                    .rt_centroiding
                    .then(RtConsistencyConfig::default),
                peak_cap: input.peak_cap,
                min_ms2_ion_count: input.min_ion_count_ms2,
                ..Config::default()
            };
            let manifest = crate::export_neighborhoods(
                &input.in_path,
                &input.out_path,
                config,
                matches!(input.sampling, Sampling::Precursor),
                input.min_ion_count_ms1,
                input.max_bundles.map(|n| n.get()),
            )?;
            println!(
                "Exported {} bundles, {} observations and {} peaks",
                manifest.bundles, manifest.observations, manifest.peaks
            );
            return Ok(());
        }
        validate_output_path(&input.out_path)
            .map_err(timsrust_centroid::TimsError::new)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acquisition_policies_are_explicit_and_neighborhood_only() {
        for policy in [
            "single-interval",
            "shared-transmission",
            "experimental-trajectory",
        ] {
            let input = Input::try_parse_from([
                "timscentroid",
                "run.d",
                "--neighborhoods",
                "--acquisition",
                policy,
            ])
            .unwrap();
            let acquisition: AcquisitionPolicy = input.acquisition.into();
            assert_eq!(
                acquisition == AcquisitionPolicy::ExperimentalTrajectory,
                policy == "experimental-trajectory"
            );
        }
        assert!(
            Input::try_parse_from([
                "timscentroid",
                "run.d",
                "--acquisition",
                "experimental-trajectory",
            ])
            .is_err()
        );
        assert!(
            Input::try_parse_from([
                "timscentroid",
                "run.d",
                "--neighborhoods",
                "--acquisition",
                "unknown",
            ])
            .is_err()
        );
    }
    #[test]
    fn legacy_invocation_and_both_rt_switch_values_parse() {
        let legacy = Input::try_parse_from([
            "timscentroid",
            "run.d",
            "-o",
            "out.parquet",
        ])
        .unwrap();
        assert!(!legacy.neighborhoods);
        for value in ["true", "false"] {
            let input = Input::try_parse_from([
                "timscentroid",
                "run.d",
                "--neighborhoods",
                "--sampling",
                "precursor",
                "--rt-centroiding",
                value,
                "-o",
                "dataset",
            ])
            .unwrap();
            assert_eq!(input.rt_centroiding, value == "true");
        }
        assert!(
            Input::try_parse_from([
                "timscentroid",
                "run.d",
                "--rt-centroiding"
            ])
            .is_err()
        );
    }
}
