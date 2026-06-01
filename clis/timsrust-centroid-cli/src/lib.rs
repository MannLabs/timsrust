//! ### Writing Peaks to Parquet (with `writer` feature)
//! ```ignore
//! use timsrust_core::io::parquet_writer::ParquetWriter;
//! use timsrust_centroid::{PeakReader, Peak};
//! let frame_reader = /* e.g. TdfFrameReader::new("example.d").unwrap() */;
//! let reader = PeakReader::new(frame_reader, 10.0, 2.0).unwrap();
//! let peaks = reader.get_peaks_from_frame(100).unwrap();
//! //let mut writer = ParquetWriter::new("output.parquet").unwrap();
//! //let written = writer.write_batch(peaks);
//! ```
//!
//! Or automated:
//! ```no_run
//! use timsrust_centroid_cli::run;
//! // Specify minimum ion counts for MS!, MS2, minimum spectrum size and whether to use precursors
//! let result = run("raw_data.d", "output.parquet", 5.0, 2.0, 5, false);
//! ```
//!
//! ```no_run
//! use timsrust_centroid_cli::run;
//! // Specify minimum ion counts for MS!, MS2, minimum spectrum size and whether to use precursors
//! let result = run("raw_data.d", "output.mgf", 5.0, 2.0, 5, false);
//! ```
//!
//! ## CLI Usage (with `cli` feature, enabled by default)
//! Run `timsrust_centroid` from the command line for processing.
//!
//! ```sh
//! timsrust_centroid --help
//! ```
//!
//! ```sh
//! timsrust_centroid input.d output.parquet --min-ion-count_ms1 5 --min-ion-count_ms2 2
//! ```
//!
//! ```sh
//! timsrust_centroid input.d output.mgf --min-ion-count_ms2 2 --min-spectrum-size 5
//! ```
//!
//! ## Features enabled by default
//! - `cli`: Enables the command-line interface
//! - `writer`: Enables Parquet output
//! - `runner`: Enables batch runner utilities

mod cli;
// mod parquet;
mod runner;

pub use cli::CLI;
pub use runner::run;
use serde::{Deserialize, Serialize};
// use timsrust_centroid::Peak;
// use timsrust_core::io::impl_parquet_scheme;

// impl_parquet_scheme!(
//     Peak,
//     [
//         (frame, arrow::datatypes::UInt32Type, false),
//         (scan, arrow::datatypes::UInt32Type, false),
//         (tof, arrow::datatypes::UInt32Type, false),
//         (apex_intensity, arrow::datatypes::UInt64Type, false),
//     ]
// );

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoordinatePeak {
    pub frame: u32,
    pub scan: u32,
    pub tof: u32,
    pub apex_intensity: u64,
    pub rt: f64,
    pub im: f64,
    pub mz: f64,
}

// impl_parquet_scheme!(
//     CoordinatePeak,
//     [
//         (frame, arrow::datatypes::UInt32Type, false),
//         (scan, arrow::datatypes::UInt32Type, false),
//         (tof, arrow::datatypes::UInt32Type, false),
//         (apex_intensity, arrow::datatypes::UInt64Type, false),
//         (rt, arrow::datatypes::Float64Type, false),
//         (im, arrow::datatypes::Float64Type, false),
//         (mz, arrow::datatypes::Float64Type, false),
//     ]
// );

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FullPeak {
    pub frame: u32,
    pub scan: u32,
    pub tof: u32,
    pub apex_intensity: u64,
    pub rt: f64,
    pub im: f64,
    pub mz: f64,
    pub isolation_window_lower: Option<f64>,
    pub isolation_window_upper: Option<f64>,
}

// impl_parquet_scheme!(
//     FullPeak,
//     [
//         (frame, arrow::datatypes::UInt32Type, false),
//         (scan, arrow::datatypes::UInt32Type, false),
//         (tof, arrow::datatypes::UInt32Type, false),
//         (apex_intensity, arrow::datatypes::UInt64Type, false),
//         (rt, arrow::datatypes::Float64Type, false),
//         (im, arrow::datatypes::Float64Type, false),
//         (mz, arrow::datatypes::Float64Type, false),
//         (isolation_window_lower, arrow::datatypes::Float64Type, true),
//         (isolation_window_upper, arrow::datatypes::Float64Type, true),
//     ]
// );

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Precursor {
    pub frame: u32,
    pub scan: u32,
    pub tof: u32,
    pub apex_intensity: u64,
    pub rt: f64,
    pub im: f64,
    pub mz: f64,
    pub start: u64,
    pub end: u64,
    pub charge: u8,
    pub index: u32,
    pub isolation_mz: f64,
    pub isolation_width: f64,
    pub ce: f64,
}

// impl_parquet_scheme!(
//     Precursor,
//     [
//         (frame, arrow::datatypes::UInt32Type, false),
//         (scan, arrow::datatypes::UInt32Type, false),
//         (tof, arrow::datatypes::UInt32Type, false),
//         (apex_intensity, arrow::datatypes::UInt64Type, false),
//         (rt, arrow::datatypes::Float64Type, false),
//         (im, arrow::datatypes::Float64Type, false),
//         (mz, arrow::datatypes::Float64Type, false),
//         (start, arrow::datatypes::UInt64Type, false),
//         (end, arrow::datatypes::UInt64Type, false),
//         (charge, arrow::datatypes::UInt8Type, false),
//         (index, arrow::datatypes::UInt32Type, false),
//         (isolation_mz, arrow::datatypes::Float64Type, false),
//         (isolation_width, arrow::datatypes::Float64Type, false),
//         (ce, arrow::datatypes::Float64Type, false),
//     ]
// );
