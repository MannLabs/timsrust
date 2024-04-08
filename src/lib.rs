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
//! * Bruker .d folder containing:
//!     * analysis.tdf
//!     * analysis.tdf_bin
//! * miniTDF - ProteoScape optimized Bruker file-format. Similar to TDF, miniTDF consists of multiple files: a binary '.bin'
//!  and an index '.parquet' file. The file-names are made up to the following convention: `<producing-engine-name>.<domain-name>.<extension>`.
//!  e.g. for MS2 spectrum information: `<producing-engine-name>.ms2spectrum.<extension>`. Therefore the following files are expected
//!  in the provided ms2 folder:
//!     * *.ms2spectrum.bin
//!     * *.ms2spectrum.parquet

mod calibration;
mod domain_converters;
mod errors;
mod file_readers;
mod ms_data;
mod utils;

pub use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter, Tof2MzConverter,
    },
    errors::*,
    file_readers::FileReader,
    ms_data::{
        AcquisitionType, Frame, FrameType, Precursor, QuadrupoleEvent, Spectrum,
    },
};
