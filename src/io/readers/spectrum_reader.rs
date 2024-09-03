#[cfg(feature = "minitdf")]
mod minitdf;
#[cfg(feature = "tdf")]
mod tdf;

use core::fmt;

#[cfg(feature = "minitdf")]
use minitdf::{MiniTDFSpectrumReader, MiniTDFSpectrumReaderError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(feature = "tdf")]
use tdf::{TDFSpectrumReader, TDFSpectrumReaderError};

use crate::ms_data::Spectrum;

#[cfg(feature = "tdf")]
use super::FrameWindowSplittingConfiguration;

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
                #[cfg(feature = "minitdf")]
                Some("ms2") => {
                    Box::new(MiniTDFSpectrumReader::new(self.path.clone())?)
                },
                #[cfg(feature = "tdf")]
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
        let mut reader = SpectrumReader { spectrum_reader };
        if self.config.spectrum_processing_params.calibrate {
            reader.calibrate();
        }
        Ok(reader)
    }
}

trait SpectrumReaderTrait: Sync + Send {
    fn get(&self, index: usize) -> Result<Spectrum, SpectrumReaderError>;
    fn get_path(&self) -> PathBuf;
    fn len(&self) -> usize;
    fn calibrate(&mut self);
}

#[derive(Debug, thiserror::Error)]
pub enum SpectrumReaderError {
    #[cfg(feature = "minitdf")]
    #[error("{0}")]
    MiniTDFSpectrumReaderError(#[from] MiniTDFSpectrumReaderError),
    #[cfg(feature = "tdf")]
    #[error("{0}")]
    TDFSpectrumReaderError(#[from] TDFSpectrumReaderError),
    #[error("File {0} not valid")]
    SpectrumReaderFileError(PathBuf),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct SpectrumProcessingParams {
    pub smoothing_window: u32,
    pub centroiding_window: u32,
    pub calibration_tolerance: f64,
    pub calibrate: bool,
}

impl Default for SpectrumProcessingParams {
    fn default() -> Self {
        Self {
            smoothing_window: 1,
            centroiding_window: 1,
            calibration_tolerance: 0.1,
            calibrate: false,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct SpectrumReaderConfig {
    pub spectrum_processing_params: SpectrumProcessingParams,
    #[cfg(feature = "tdf")]
    pub frame_splitting_params: FrameWindowSplittingConfiguration,
}
