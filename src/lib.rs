//! This crate allows to read Bruker TimsTOF data.
//!
//! ## Basics
//!
//! Two primary data types are exposed:
//!
//! * [Spectra](crate::ms_data::Spectrum): A traditional representation that expresses intensitites in function of mz values for a given precursor.
//! * [Frames](crate::ms_data::Frame): All recorded data from a single TIMS elution (i.e. at one specific retention_time).
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

pub(crate) mod domain_converters;
pub(crate) mod errors;
pub(crate) mod io;
pub(crate) mod ms_data;
pub(crate) mod utils;

pub mod converters {
    //! Allows conversions between domains (e.g. Time of Flight and m/z)
    pub use crate::domain_converters::*;
}
pub mod readers {
    //! Readers for all data from Bruker compatible files.
    pub use crate::io::readers::*;
}
pub mod writers {
    //! Writers to generic file formats.
    pub use crate::io::writers::*;
}
pub use crate::errors::*;
pub use crate::ms_data::*;
