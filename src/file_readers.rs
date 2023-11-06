use crate::{
    calibration::Tof2MzCalibrator,
    converters::{Frame2RtConverter, Scan2ImConverter, Tof2MzConverter},
    Error,
};

mod common;
mod file_formats;
mod frame_readers;
mod spectrum_readers;

use {
    self::{
        file_formats::FileFormat, frame_readers::ReadableFrames,
        spectrum_readers::ReadableSpectra,
    },
    crate::{Frame, Spectrum},
};

pub use file_formats::FileFormatError;

use self::frame_readers::tdf_reader::TDFReader;

/// A reader to read [frames](crate::Frame) and [spectra](crate::Spectrum).
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

    pub fn read_all_ms1_frames(&self) -> Vec<Frame> {
        self.format.read_all_ms1_frames()
    }

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

    pub fn get_frame_converter(&self) -> Result<Frame2RtConverter, Error> {
        match &self.format {
            FileFormat::DFolder(path) => Ok(TDFReader::new(
                &path.to_str().unwrap_or_default().to_string(),
            )
            .rt_converter),
            _ => Err(Error::FileFormatError(
                FileFormatError::MetadataFilesAreMissing,
            )),
        }
    }

    pub fn get_scan_converter(&self) -> Result<Scan2ImConverter, Error> {
        match &self.format {
            FileFormat::DFolder(path) => Ok(TDFReader::new(
                &path.to_str().unwrap_or_default().to_string(),
            )
            .im_converter),
            _ => Err(Error::FileFormatError(
                FileFormatError::MetadataFilesAreMissing,
            )),
        }
    }

    pub fn get_tof_converter(&self) -> Result<Tof2MzConverter, Error> {
        match &self.format {
            FileFormat::DFolder(path) => Ok(TDFReader::new(
                &path.to_str().unwrap_or_default().to_string(),
            )
            .mz_converter),
            _ => Err(Error::FileFormatError(
                FileFormatError::MetadataFilesAreMissing,
            )),
        }
    }
}
