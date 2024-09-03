#[cfg(feature = "minitdf")]
mod minitdf;
#[cfg(feature = "tdf")]
mod tdf;

use core::fmt;
use std::path::{Path, PathBuf};

#[cfg(feature = "minitdf")]
use minitdf::{MiniTDFPrecursorReader, MiniTDFPrecursorReaderError};
#[cfg(feature = "tdf")]
use tdf::{TDFPrecursorReader, TDFPrecursorReaderError};

use crate::ms_data::Precursor;

#[cfg(feature = "tdf")]
use super::FrameWindowSplittingConfiguration;

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
    #[cfg(feature = "tdf")]
    config: FrameWindowSplittingConfiguration,
}

impl PrecursorReaderBuilder {
    pub fn with_path(&self, path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
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

    pub fn finalize(&self) -> Result<PrecursorReader, PrecursorReaderError> {
        let precursor_reader: Box<dyn PrecursorReaderTrait> =
            match self.path.extension().and_then(|e| e.to_str()) {
                #[cfg(feature = "minitdf")]
                Some("parquet") => {
                    Box::new(MiniTDFPrecursorReader::new(self.path.clone())?)
                },
                #[cfg(feature = "tdf")]
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
    #[error("File {0} not valid")]
    PrecursorReaderFileError(PathBuf),
}
