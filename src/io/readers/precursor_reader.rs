mod minitdf;
mod tdf;

use core::fmt;
use std::path::{Path, PathBuf};

use minitdf::{MiniTDFPrecursorReader, MiniTDFPrecursorReaderError};
use tdf::{TDFPrecursorReader, TDFPrecursorReaderError};

use crate::ms_data::Precursor;

use super::quad_settings_reader::FrameWindowSplittingStrategy;

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

    pub fn new(path: impl AsRef<Path>) -> Result<Self, PrecursorReaderError> {
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
    path: PathBuf,
    config: FrameWindowSplittingStrategy,
}

impl PrecursorReaderBuilder {
    pub fn with_path(&self, path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            ..self.clone()
        }
    }

    pub fn with_config(&self, config: FrameWindowSplittingStrategy) -> Self {
        Self {
            config: config,
            ..self.clone()
        }
    }

    pub fn finalize(&self) -> Result<PrecursorReader, PrecursorReaderError> {
        let precursor_reader: Box<dyn PrecursorReaderTrait> =
            match self.path.extension().and_then(|e| e.to_str()) {
                Some("parquet") => {
                    Box::new(MiniTDFPrecursorReader::new(self.path.clone())?)
                },
                Some("tdf") => Box::new(TDFPrecursorReader::new(
                    self.path.clone(),
                    self.config.clone(),
                )?),
                _ => {
                    return Err(PrecursorReaderError::PrecursorReaderFileError(
                        self.path.clone(),
                    ))
                },
            };
        let reader = PrecursorReader { precursor_reader };
        Ok(reader)
    }
}

trait PrecursorReaderTrait: Sync {
    fn get(&self, index: usize) -> Option<Precursor>;
    fn len(&self) -> usize;
}

#[derive(Debug, thiserror::Error)]
pub enum PrecursorReaderError {
    #[error("{0}")]
    MiniTDFPrecursorReaderError(#[from] MiniTDFPrecursorReaderError),
    #[error("{0}")]
    TDFPrecursorReaderError(#[from] TDFPrecursorReaderError),
    #[error("File {0} not valid")]
    PrecursorReaderFileError(PathBuf),
}
