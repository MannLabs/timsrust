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

pub struct FileReader {
    format: FileFormat,
}

impl FileReader {
    pub fn new<T: AsRef<std::path::Path>>(path_name: T) -> Self {
        let format: FileFormat = FileFormat::parse(path_name);
        Self { format }
    }

    pub fn read_all_frames(&self) -> Vec<Frame> {
        self.format.read_all_frames()
    }

    pub fn read_all_spectra(&self) -> Vec<Spectrum> {
        self.format.read_all_spectra()
    }
}
