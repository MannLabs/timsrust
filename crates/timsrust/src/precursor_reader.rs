use std::sync::Arc;

#[cfg(feature = "bps")]
use timsrust_bruker::parquet_spectra::precursor_reader::{
    ParquetPrecursorReader, ParquetPrecursorReaderError,
};
use timsrust_core::Precursor;
use timsrust_core::utils::reader::Reader;
use timsrust_minitdf::MiniTDFError;
use timsrust_minitdf::MiniTDFPrecursorReader;
use timsrust_tdf::{
    FrameWindowSplittingConfiguration, TDFPrecursorReader,
    TDFPrecursorReaderError,
};

use crate::{
    ImConverter, TimsTofPath, TimsTofPathError, TimsTofPathLike,
    timstof::TimsTofFileType,
};

enum Inner {
    MiniTDF(MiniTDFPrecursorReader),
    Tdf(TDFPrecursorReader<ImConverter>),
    #[cfg(feature = "bps")]
    ParquetSpectra(ParquetPrecursorReader),
}

impl Inner {
    fn get(&self, index: usize) -> Result<Precursor, PrecursorReaderError> {
        match self {
            Inner::MiniTDF(reader) => Ok(reader.get(index)?),
            Inner::Tdf(reader) => Ok(reader.get(index)?),
            #[cfg(feature = "bps")]
            Inner::ParquetSpectra(reader) => Ok(reader.get(index)?),
        }
    }

    fn len(&self) -> usize {
        match self {
            Inner::MiniTDF(reader) => reader.len(),
            Inner::Tdf(reader) => reader.len(),
            #[cfg(feature = "bps")]
            Inner::ParquetSpectra(reader) => reader.len(),
        }
    }
}

pub struct PrecursorReader {
    precursor_reader: Inner,
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

    pub fn get(&self, index: usize) -> Result<Precursor, PrecursorReaderError> {
        self.precursor_reader.get(index)
    }

    pub fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Default, Clone)]
pub struct PrecursorReaderBuilder {
    path: Option<TimsTofPath>,
    config: FrameWindowSplittingConfiguration<ImConverter>,
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
    pub fn with_config(
        &self,
        config: FrameWindowSplittingConfiguration<ImConverter>,
    ) -> Self {
        Self {
            config,
            ..self.clone()
        }
    }

    pub fn finalize(self) -> Result<PrecursorReader, PrecursorReaderError> {
        let path = match self.path {
            None => return Err(PrecursorReaderError::NoPath),
            Some(path) => path,
        };
        let precursor_reader = match path.file_type() {
            TimsTofFileType::MiniTdf(mini_path) => {
                Inner::MiniTDF(mini_path.precursor_reader()?)
            },
            TimsTofFileType::Tdf(tdf_path) => {
                let im_converter = Arc::new(ImConverter::new(&path).unwrap());
                Inner::Tdf(TDFPrecursorReader::new(
                    tdf_path,
                    self.config,
                    im_converter,
                )?)
            },
            #[cfg(feature = "bps")]
            TimsTofFileType::Parquet(parquet_path) => Inner::ParquetSpectra(
                ParquetPrecursorReader::new(parquet_path.precursor_path()),
            ),
        };
        let reader = PrecursorReader { precursor_reader };
        Ok(reader)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrecursorReaderError {
    #[error("{0}")]
    MiniTDFPrecursorReaderError(#[from] MiniTDFError),
    #[cfg(feature = "bps")]
    #[error("{0}")]
    ParquetPrecursorReader(#[from] ParquetPrecursorReaderError),
    #[error("{0}")]
    TDFPrecursorReaderError(#[from] TDFPrecursorReaderError),
    #[error("No path provided")]
    NoPath,
    #[error("{0}")]
    TimsTofPathError(#[from] TimsTofPathError),
}
