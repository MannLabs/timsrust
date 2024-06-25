mod minitdf;
mod tdf;

use core::fmt;
use std::path::{Path, PathBuf};

use minitdf::MiniTDFSpectrumReader;
use tdf::TDFSpectrumReader;

use crate::ms_data::Spectrum;

pub struct SpectrumReader {
    spectrum_reader: Box<dyn SpectrumReaderTrait>,
}

impl fmt::Debug for SpectrumReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpectrumReader {{ /* fields omitted */ }}")
    }
}

impl SpectrumReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let spectrum_reader: Box<dyn SpectrumReaderTrait> =
            match path.as_ref().extension().and_then(|e| e.to_str()) {
                Some("parquet") => Box::new(MiniTDFSpectrumReader::new(path)),
                Some("tdf") => Box::new(TDFSpectrumReader::new(path)),
                _ => panic!(),
            };
        Self { spectrum_reader }
    }

    pub fn get(&self, index: usize) -> Spectrum {
        self.spectrum_reader.get(index)
    }

    pub fn get_path(&self) -> PathBuf {
        self.spectrum_reader.get_path()
    }

    pub fn len(&self) -> usize {
        self.spectrum_reader.len()
    }
}

trait SpectrumReaderTrait: Sync {
    fn get(&self, index: usize) -> Spectrum;
    fn get_path(&self) -> PathBuf;
    fn len(&self) -> usize;
}
