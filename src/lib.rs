//! This crate allows to read Bruker TimsTOF data.
//!
//! ## Basics
//!
//! Two primary data types are exposed:
//!
//! * [Spectra](crate::Spectrum): A traditional representation that expresses intensitites in function of mz values for a given precursor.
//! * [Frames](crate::Frame): All recorded data from a single TIMS elution (i.e. at one specific retention_time).
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
pub mod file_readers;
mod frames;
mod precursors;
mod spectra;
mod vec_utils;

pub use file_readers::ReadableFrames;

pub use crate::converters::{
    ConvertableIndex, Frame2RtConverter, Scan2ImConverter, Tof2MzConverter,
};

pub use crate::{
    acquisition::AcquisitionType,
    errors::*,
    file_readers::{FileReader, TDFReader},
    frames::{Frame, FrameType, FrameMSMSWindow},
    precursors::{Precursor, PrecursorType},
    spectra::Spectrum,
};
