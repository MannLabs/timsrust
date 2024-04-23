use crate::ms_data::Spectrum;

use self::{dda_reader::DDASpectrumReader, mini_tdf_reader::MiniTDFReader};

use super::file_formats::FileFormat;

pub mod dda_reader;
pub mod mini_tdf_reader;

pub trait ReadableSpectra {
    fn read_single_spectrum(&self, index: usize) -> Spectrum;

    fn read_all_spectra(&self) -> Vec<Spectrum>;
}

impl FileFormat {
    fn unwrap_spectrum_reader(&self) -> Box<dyn ReadableSpectra> {
        let result = match &self {
            Self::DFolder(path) => Box::new(DDASpectrumReader::new(
                path.to_str().unwrap_or_default().to_string(),
            )) as Box<dyn ReadableSpectra>,
            Self::MS2Folder(path) => Box::new(MiniTDFReader::new(
                path.to_str().unwrap_or_default().to_string(),
            )) as Box<dyn ReadableSpectra>,
        };
        result
    }
}

impl ReadableSpectra for FileFormat {
    fn read_single_spectrum(&self, index: usize) -> Spectrum {
        self.unwrap_spectrum_reader().read_single_spectrum(index)
    }

    fn read_all_spectra(&self) -> Vec<Spectrum> {
        self.unwrap_spectrum_reader().read_all_spectra()
    }
}
