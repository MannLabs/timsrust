//! This crate allows to read Bruker TimsTOF data.
//!
//! ## Basics
//!
//! Two primary data types are exposed:
//!
//! * Spectra: A traditional representation that expresses intensitites in function of mz values for a given precursor.
//! * Frames: All recorded data from a single TIMS elution (i.e. at one specific retention_time).
//!
//! ## File formats
//!
//! Two file formats are supported:
//!
//! * Bruker .ms2 folder containing:
//!     * converter.ms2.bin
//!     * converter.MS2Spectra.ms2.parquet
//! * Bruker .d folder containing:
//!     * analysis.tdf
//!     * analysis.tdf_bin

mod acquisition;
mod calibration;
mod converters;
mod errors;
mod file_readers;
mod frames;
mod precursors;
mod spectra;
mod vec_utils;

pub use crate::{
    acquisition::AcquisitionType,
    errors::*,
    file_readers::FileReader,
    frames::{Frame, FrameType},
    precursors::{Precursor, PrecursorType},
    spectra::{RawSpectrum, Spectrum},
};
