mod builder;
mod config;
mod errors;
#[cfg(feature = "minitdf")]
mod minitdf;
mod spectrum_trait;
#[cfg(feature = "tdf")]
mod tdf;

use super::TimsTofPathLike;
use crate::ms_data::Spectrum;
pub use builder::SpectrumReaderBuilder;
pub use config::{SpectrumProcessingParams, SpectrumReaderConfig};
pub use errors::SpectrumReaderError;
use rayon::prelude::*;
use spectrum_trait::SpectrumReaderTrait;

pub struct SpectrumReader {
    spectrum_reader: Box<dyn SpectrumReaderTrait>,
}

impl SpectrumReader {
    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Self, SpectrumReaderError> {
        Self::build().with_path(path).finalize()
    }

    pub fn build() -> SpectrumReaderBuilder {
        SpectrumReaderBuilder::default()
    }

    pub fn get(&self, index: usize) -> Result<Spectrum, SpectrumReaderError> {
        self.spectrum_reader.get(index)
    }

    pub fn len(&self) -> usize {
        self.spectrum_reader.len()
    }

    pub fn get_all(&self) -> Vec<Result<Spectrum, SpectrumReaderError>> {
        let mut spectra: Vec<Result<Spectrum, SpectrumReaderError>> = (0..self
            .len())
            .into_par_iter()
            .map(|index| self.get(index))
            .collect();
        spectra.sort_by_key(|x| match x {
            Ok(spectrum) => match spectrum.precursor {
                Some(precursor) => precursor.index,
                None => spectrum.index,
            },
            Err(_) => 0,
        });
        spectra
    }

    fn calibrate(&mut self) {
        self.spectrum_reader.calibrate();
    }
}
