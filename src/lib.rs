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

mod calibration;
mod converters;
mod file_readers;
mod frames;
mod precursors;
mod spectra;
mod vec_utils;

pub use crate::{
    file_readers::FileReader,
    frames::{Frame, FrameType},
    precursors::{Precursor, PrecursorType},
    spectra::{RawSpectrum, Spectrum},
};

#[derive(Debug)]
pub enum Error {
    UnknownFileFormat
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownFileFormat => f.write_str("unknown file format"),
        }
    }
}

impl std::error::Error for Error {}