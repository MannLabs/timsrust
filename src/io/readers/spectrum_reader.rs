mod minitdf;
mod tdf;

use core::fmt;
use minitdf::MiniTDFSpectrumReader;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};
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
                Some("ms2") => Box::new(MiniTDFSpectrumReader::new(path)),
                Some("d") => Box::new(TDFSpectrumReader::new(path)),
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

    pub fn get_all(&self) -> Vec<Spectrum> {
        let mut spectra: Vec<Spectrum> = (0..self.len())
            .into_par_iter()
            .map(|index| self.get(index))
            .collect();
        spectra.sort_by_key(|x| x.precursor.index);
        spectra
    }

    pub fn calibrate(&mut self) {
        self.spectrum_reader.calibrate();
    }
}

trait SpectrumReaderTrait: Sync {
    fn get(&self, index: usize) -> Spectrum;
    fn get_path(&self) -> PathBuf;
    fn len(&self) -> usize;
    fn calibrate(&mut self);
}
