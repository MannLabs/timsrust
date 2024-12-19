#[cfg(feature = "minitdf")]
mod minitdf;
#[cfg(feature = "tdf")]
mod tdf;

use core::fmt;

#[cfg(feature = "minitdf")]
use minitdf::{MiniTDFPrecursorReader, MiniTDFPrecursorReaderError};
#[cfg(feature = "tdf")]
use tdf::{TDFPrecursorReader, TDFPrecursorReaderError};

use crate::ms_data::Precursor;

#[cfg(feature = "tdf")]
use super::FrameWindowSplittingConfiguration;
use super::{TimsTofFileType, TimsTofPath, TimsTofPathError, TimsTofPathLike};

pub struct PrecursorReader {
    precursor_reader: Box<dyn PrecursorReaderTrait>,
}

impl fmt::Debug for PrecursorReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrecursorReader {{ /* fields omitted */ }}")
    }
}

impl PrecursorReader {
    pub fn build() -> PrecursorReaderBuilder {
        PrecursorReaderBuilder::default()
    }

    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Self, PrecursorReaderError> {
        Self::build().with_path(path).finalize()
    }

    pub fn get(&self, index: usize) -> Option<Precursor> {
        self.precursor_reader.get(index)
    }

    pub fn len(&self) -> usize {
        self.precursor_reader.len()
    }
}

#[derive(Debug, Default, Clone)]
pub struct PrecursorReaderBuilder {
    path: Option<TimsTofPath>,
    #[cfg(feature = "tdf")]
    config: FrameWindowSplittingConfiguration,
}

impl PrecursorReaderBuilder {
    pub fn with_path(&self, path: impl TimsTofPathLike) -> Self {
        // TODO
        let path = Some(path.to_timstof_path().unwrap());
        Self {
            path,
            ..self.clone()
        }
    }

    #[cfg(feature = "tdf")]
    pub fn with_config(
        &self,
        config: FrameWindowSplittingConfiguration,
    ) -> Self {
        Self {
            config: config,
            ..self.clone()
        }
    }

    pub fn finalize(self) -> Result<PrecursorReader, PrecursorReaderError> {
        let path = match self.path {
            None => return Err(PrecursorReaderError::NoPath),
            Some(path) => path,
        };
        let precursor_reader: Box<dyn PrecursorReaderTrait> =
            match path.file_type() {
                #[cfg(feature = "minitdf")]
                TimsTofFileType::MiniTDF => {
                    Box::new(MiniTDFPrecursorReader::new(path)?)
                },
                #[cfg(feature = "tdf")]
                TimsTofFileType::TDF => {
                    Box::new(TDFPrecursorReader::new(path, self.config)?)
                },
            };
        let reader = PrecursorReader { precursor_reader };
        Ok(reader)
    }
}

trait PrecursorReaderTrait: Sync + Send {
    fn get(&self, index: usize) -> Option<Precursor>;
    fn len(&self) -> usize;
}

#[derive(Debug, thiserror::Error)]
pub enum PrecursorReaderError {
    #[cfg(feature = "minitdf")]
    #[error("{0}")]
    MiniTDFPrecursorReaderError(#[from] MiniTDFPrecursorReaderError),
    #[cfg(feature = "tdf")]
    #[error("{0}")]
    TDFPrecursorReaderError(#[from] TDFPrecursorReaderError),
    #[error("No path provided")]
    NoPath,
    #[error("{0}")]
    TimsTofPathError(#[from] TimsTofPathError),
}
