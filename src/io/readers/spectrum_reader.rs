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

impl fmt::Debug for SpectrumReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpectrumReader {{ /* fields omitted */ }}")
    }
}

impl SpectrumReader {
    pub fn build() -> SpectrumReaderBuilder {
        SpectrumReaderBuilder::default()
    }

    pub fn new(path: impl AsRef<Path>) -> Result<Self, SpectrumReaderError> {
        Self::build().with_path(path).finalize()
    }

    pub fn get(&self, index: usize) -> Result<Spectrum, SpectrumReaderError> {
        self.spectrum_reader.get(index)
    }

    pub fn get_path(&self) -> PathBuf {
        self.spectrum_reader.get_path()
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
        spectra.sort_by_key(|x| x.as_ref().unwrap().precursor.unwrap().index);
        spectra
    }

    pub fn calibrate(&mut self) {
        self.spectrum_reader.calibrate();
    }
}

#[derive(Debug, Default, Clone)]
pub struct SpectrumReaderBuilder {
    path: PathBuf,
    config: SpectrumReaderConfig,
}

impl SpectrumReaderBuilder {
    pub fn with_path(&self, path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            ..self.clone()
        }
    }

    pub fn with_config(&self, config: SpectrumReaderConfig) -> Self {
        Self {
            config: config,
            ..self.clone()
        }
    }

    pub fn finalize(&self) -> Result<SpectrumReader, SpectrumReaderError> {
        let spectrum_reader: Box<dyn SpectrumReaderTrait> =
            match self.path.extension().and_then(|e| e.to_str()) {
                Some("ms2") => {
                    Box::new(MiniTDFSpectrumReader::new(self.path.clone())?)
                },
                Some("d") => Box::new(TDFSpectrumReader::new(
                    self.path.clone(),
                    self.config.clone(),
                )?),
                _ => {
                    return Err(SpectrumReaderError::SpectrumReaderFileError(
                        self.path.clone(),
                    ))
                },
            };
        let reader = SpectrumReader { spectrum_reader };
        Ok(reader)
    }
}

trait SpectrumReaderTrait: Sync {
    fn get(&self, index: usize) -> Result<Spectrum, SpectrumReaderError>;
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
    #[error("File {0} not valid")]
    SpectrumReaderFileError(PathBuf),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
pub enum FrameWindowSplittingStrategy {
    Quadrupole(QuadWindowExpansionStrategy),
    Window(QuadWindowExpansionStrategy),
}

impl Default for FrameWindowSplittingStrategy {
    fn default() -> Self {
        Self::Quadrupole(QuadWindowExpansionStrategy::Even(1))
    }
}

#[derive(Debug, Default, Clone)]
pub struct SpectrumReaderConfig {
    pub spectrum_processing_params: SpectrumProcessingParams,
    pub frame_splitting_params: FrameWindowSplittingStrategy,
}
