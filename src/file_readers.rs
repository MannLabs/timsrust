use crate::Error;

mod file_formats;
mod spectrum_readers;

use {
    self::{file_formats::FileFormat, spectrum_readers::ReadableSpectra},
    crate::ms_data::{Frame, Spectrum},
};

pub use file_formats::FileFormatError;

/// A reader to read [frames](crate::ms_data::Frame) and [spectra](crate::ms_data::Spectrum).
pub struct FileReader {
    format: FileFormat,
}

///NOTE: The functions to read a single frame or spectrum are not optimized.
/// In case many frames or spectra are required, it is best to use
/// any of the functions that directly return a `Vec`.
impl FileReader {
    pub fn new<T: AsRef<std::path::Path>>(path_name: T) -> Result<Self, Error> {
        let format: FileFormat = FileFormat::parse(path_name)?;
        Ok(Self { format })
    }

    pub fn read_single_frame(&self, index: usize) -> Frame {
        self.format.read_single_frame(index)
    }

    pub fn read_all_frames(&self) -> Vec<Frame> {
        self.format.read_all_frames()
    }

    /// NOTE: The returned vec contains all frames to not disrupt indexing.
    /// MS2 frames are set to unknown and not read.
    pub fn read_all_ms1_frames(&self) -> Vec<Frame> {
        self.format.read_all_ms1_frames()
    }

    /// NOTE: The returned vec contains all frames to not disrupt indexing.
    /// MS1 frames are set to unknown and not read.
    pub fn read_all_ms2_frames(&self) -> Vec<Frame> {
        self.format.read_all_ms2_frames()
    }

    pub fn read_single_spectrum(&self, index: usize) -> Spectrum {
        self.format.read_single_spectrum(index)
    }

    ///NOTE: ddaPASEF MS2 spectra are automatically calibrated with
    /// all unfragmented precursor signals.
    /// Hence, reading spectra individually through `read_single_spectrum`
    /// might yield slightly different mz values.
    pub fn read_all_spectra(&self) -> Vec<Spectrum> {
        self.format.read_all_spectra()
    }
}
