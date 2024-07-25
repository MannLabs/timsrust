mod minitdf;
mod tdf;

use core::fmt;
use minitdf::{MiniTDFSpectrumReader, MiniTDFSpectrumReaderError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use tdf::{TDFSpectrumReader, TDFSpectrumReaderError};

use crate::io::readers::tdf_utils::QuadWindowExpansionStrategy;
use crate::ms_data::Spectrum;

pub struct SpectrumReader {
    spectrum_reader: Box<dyn SpectrumReaderTrait>,
}

#[derive(Debug)]
pub struct SpectrumProcessingParams {
    smoothing_window: u32,
    centroiding_window: u32,
    calibration_tolerance: f64,
}

impl Default for SpectrumProcessingParams {
    fn default() -> Self {
        Self {
            smoothing_window: 1,
            centroiding_window: 1,
            calibration_tolerance: 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum FrameWindowSplittingStrategy {
    #[default]
    None,
    Quadrupole(QuadWindowExpansionStrategy),
    Window(QuadWindowExpansionStrategy),
}

#[derive(Debug, Default)]
pub struct SpectrumReaderConfig {
    pub spectrum_processing_params: SpectrumProcessingParams,
    pub frame_splitting_params: FrameWindowSplittingStrategy,
}

impl fmt::Debug for SpectrumReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpectrumReader {{ /* fields omitted */ }}")
    }
}

impl SpectrumReader {
    pub fn new(
        path: impl AsRef<Path>,
        config: SpectrumReaderConfig,
    ) -> Result<Self, SpectrumReaderError> {
        let spectrum_reader: Box<dyn SpectrumReaderTrait> =
            match path.as_ref().extension().and_then(|e| e.to_str()) {
                Some("ms2") => Box::new(MiniTDFSpectrumReader::new(path)?),
                Some("d") => Box::new(TDFSpectrumReader::new(path, config)?),
                _ => panic!(),
            };
        let reader = Self { spectrum_reader };
        Ok(reader)
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
        spectra.sort_by_key(|x| x.precursor.unwrap().index);
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

#[derive(Debug, thiserror::Error)]
pub enum SpectrumReaderError {
    #[error("{0}")]
    MiniTDFSpectrumReaderError(#[from] MiniTDFSpectrumReaderError),
    #[error("{0}")]
    TDFSpectrumReaderError(#[from] TDFSpectrumReaderError),
}
