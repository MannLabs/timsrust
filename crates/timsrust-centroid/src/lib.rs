//! # timsrust_centroid
//!
//! `timsrust_centroid` provides fast centroiding and peak picking for Bruker timsTOF data.
//!
//! ## Features
//! - Fast centroiding algorithms
//! - Optional CLI and Parquet writer
//!
//! ## Examples
//!
//! ### Creating and using a PeakReader
//! ```ignore
//! use timsrust_centroid::{PeakReader, Peak};
//! // Construct a FrameReader from a format-specific crate (e.g. timsrust-tdf)
//! // and pass it together with minimum ion counts for MS1 and MS2.
//! let frame_reader = /* e.g. TdfFrameReader::new("example.d").unwrap() */;
//! let reader = PeakReader::new(frame_reader, 10.0, 5.0).unwrap();
//! let peaks = reader.get_peaks_from_frame(100).unwrap();
//! for peak in peaks.iter() {
//!     println!("peak: {:?}", peak);
//! }
//! ```
//!
//! ### Creating and using a SpectrumReader
//! ```ignore
//! use timsrust_centroid::spectrum_reader::SpectrumReader;
//! // Construct a FrameReader and converters from format-specific crates (e.g. timsrust-tdf).
//! // Pass minimum ion counts for MS1, MS2, minimum spectrum size, precursor flag, and converters.
//! let frame_reader = /* e.g. TdfFrameReader::new("example.d").unwrap() */;
//! let im_converter = /* e.g. from TDF calibration */;
//! let mz_converter = /* e.g. from TDF calibration */;
//! let reader = SpectrumReader::new(frame_reader, 10.0, 2.0, 5, false, im_converter, mz_converter).unwrap();
//! let spectra = reader.get_spectra_from_frame(100);
//! for spectrum in spectra.iter() {
//!     println!("spectrum: {:?}", spectrum);
//! }
//! ```
//!
//! ### Error handling
//! ```no_run
//! use timsrust_centroid::{TimsResult, TimsError};
//! fn do_something() -> TimsResult<()> {
//!     // ... your code ...
//!     Ok(())
//! }
//! ```
//!
mod buffer;
mod centroider;
mod error;
mod peakbuffer;
mod peaks;
// mod runner;
mod smoothing;
pub mod spectrum_reader;

pub use error::{TimsError, TimsResult};
pub use peaks::{
    Peak, PeakReader, get_average_ms1_peak, get_best_peak_for_frame,
};
// pub use runner::run;
