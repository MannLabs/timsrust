#[cfg(feature = "minitdf")]
mod minitdf;
#[cfg(feature = "tdf")]
mod tdf;

#[cfg(feature = "minitdf")]
use minitdf::{MiniTDFSpectrumReader, MiniTDFSpectrumReaderError};
use rayon::prelude::*;
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "tdf")]
use tdf::{TDFSpectrumReader, TDFSpectrumReaderError};

use crate::ms_data::Spectrum;

#[cfg(feature = "tdf")]
use super::FrameWindowSplittingConfiguration;
use super::{TimsTofFileType, TimsTofPath, TimsTofPathLike};

pub struct SpectrumReader {
    spectrum_reader: Box<dyn SpectrumReaderTrait>,
}

impl SpectrumReader {
    pub fn build() -> SpectrumReaderBuilder {
        SpectrumReaderBuilder::default()
    }

    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Self, SpectrumReaderError> {
        Self::build().with_path(path).finalize()
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

#[derive(Debug, Default, Clone)]
pub struct SpectrumReaderBuilder {
    path: Option<TimsTofPath>,
    config: SpectrumReaderConfig,
}

impl SpectrumReaderBuilder {
    pub fn with_path(&self, path: impl TimsTofPathLike) -> Self {
        // TODO
        let path = Some(path.to_timstof_path().unwrap());
        Self {
            path,
            ..self.clone()
        }
    }

    pub fn with_config(&self, config: SpectrumReaderConfig) -> Self {
        Self {
            config: config,
            ..self.clone()
        }
    }

    pub fn finalize(self) -> Result<SpectrumReader, SpectrumReaderError> {
        let path = match self.path {
            None => return Err(SpectrumReaderError::NoPath),
            Some(path) => path,
        };
        let spectrum_reader: Box<dyn SpectrumReaderTrait> =
            match path.file_type() {
                #[cfg(feature = "minitdf")]
                TimsTofFileType::MiniTDF => {
                    Box::new(MiniTDFSpectrumReader::new(path)?)
                },
                #[cfg(feature = "tdf")]
                TimsTofFileType::TDF => {
                    Box::new(TDFSpectrumReader::new(path, self.config)?)
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
    #[error("No path provided")]
    NoPath,
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
