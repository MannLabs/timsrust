mod dda;
mod dia;

use std::sync::Arc;

use dda::{DDATDFPrecursorReader, DDATDFPrecursorReaderError};
use dia::{DIATDFPrecursorReader, DIATDFPrecursorReaderError};
use timsrust_core::utils::reader::Reader;
use timsrust_core::{
    AcquisitionType, Im, InvertibleConverter, Precursor, ScanIndex,
};

use crate::{
    FrameWindowSplittingConfiguration, TDFPathLike,
    file_readers::sql_reader::{
        ReadableSqlTable, SqlReader, SqlReaderError, frames::SqlFrame,
    },
};

use super::{TDFPath, TDFPathError};

pub(crate) struct PrecursorReaderBuilder<ImC> {
    path: Option<TDFPath>,
    config: FrameWindowSplittingConfiguration<ImC>,
    im_converter: Option<Arc<ImC>>,
}

impl<ImC> Default for PrecursorReaderBuilder<ImC> {
    fn default() -> Self {
        Self {
            path: None,
            config: FrameWindowSplittingConfiguration::default(),
            im_converter: None,
        }
    }
}

impl<ImC> Clone for PrecursorReaderBuilder<ImC> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            config: self.config.clone(),
            im_converter: self.im_converter.clone(),
        }
    }
}

impl<ImC> PrecursorReaderBuilder<ImC> {
    pub(crate) fn with_path(&self, path: impl TDFPathLike) -> Self {
        let path = Some(path.to_timstof_path().unwrap());
        Self {
            path,
            ..self.clone()
        }
    }

    pub(crate) fn with_config(
        &self,
        config: FrameWindowSplittingConfiguration<ImC>,
    ) -> Self {
        Self {
            config,
            ..self.clone()
        }
    }

    pub(crate) fn with_im_converter(&self, im_converter: Arc<ImC>) -> Self {
        Self {
            im_converter: Some(im_converter),
            ..self.clone()
        }
    }
}

impl<ImC: InvertibleConverter<ScanIndex, Im> + Send + Sync + std::fmt::Debug>
    PrecursorReaderBuilder<ImC>
{
    pub(crate) fn finalize(
        self,
    ) -> Result<TDFPrecursorReader<ImC>, TDFPrecursorReaderError> {
        let path = match self.path {
            None => return Err(TDFPrecursorReaderError::NoPath),
            Some(path) => path,
        };
        let im_converter = match self.im_converter {
            None => return Err(TDFPrecursorReaderError::NoImConverter),
            Some(c) => c,
        };
        TDFPrecursorReader::new(path, self.config, im_converter)
    }
}

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
enum InnerPrecursorReader<ImC> {
    DDAPASEF(DDATDFPrecursorReader<ImC>),
    DIAPASEF(DIATDFPrecursorReader<ImC>),
}

impl<ImC: InvertibleConverter<ScanIndex, Im>> InnerPrecursorReader<ImC> {
    fn len(&self) -> usize {
        match self {
            InnerPrecursorReader::DDAPASEF(reader) => reader.len(),
            InnerPrecursorReader::DIAPASEF(reader) => reader.len(),
        }
    }
}

#[derive(Debug)]
pub struct TDFPrecursorReader<ImC> {
    precursor_reader: InnerPrecursorReader<ImC>,
}

impl<ImC: InvertibleConverter<ScanIndex, Im> + Send + Sync + std::fmt::Debug>
    TDFPrecursorReader<ImC>
{
    pub(crate) fn build() -> PrecursorReaderBuilder<ImC> {
        PrecursorReaderBuilder::default()
    }

    pub fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn new(
        path: impl TDFPathLike,
        splitting_strategy: FrameWindowSplittingConfiguration<ImC>,
        im_converter: Arc<ImC>,
    ) -> Result<Self, TDFPrecursorReaderError> {
        let tdf_sql_reader = SqlReader::open(&path)?;
        let sql_frames = SqlFrame::from_sql_reader(&tdf_sql_reader)?;
        let acquisition_type = if sql_frames.iter().any(|f| f.scan_mode == 8) {
            AcquisitionType::DDAPASEF
        } else if sql_frames.iter().any(|f| f.scan_mode == 9) {
            AcquisitionType::DIAPASEF
        } else {
            AcquisitionType::Unknown
        };
        let precursor_reader = match acquisition_type {
            AcquisitionType::DDAPASEF => InnerPrecursorReader::DDAPASEF(
                DDATDFPrecursorReader::new(&path, im_converter)?,
            ),
            AcquisitionType::DIAPASEF => {
                InnerPrecursorReader::DIAPASEF(DIATDFPrecursorReader::new(
                    path,
                    splitting_strategy,
                    im_converter,
                )?)
            },
            acquisition_type => {
                return Err(TDFPrecursorReaderError::UnsupportedAcquisition(
                    format!("{:?}", acquisition_type),
                ));
            },
        };
        Ok(Self { precursor_reader })
    }
}

impl<ImC: InvertibleConverter<ScanIndex, Im>> Reader<Precursor>
    for TDFPrecursorReader<ImC>
{
    type Error = TDFPrecursorReaderError;
    fn get(&self, index: usize) -> Result<Precursor, Self::Error> {
        match self.precursor_reader {
            InnerPrecursorReader::DDAPASEF(ref reader) => {
                Ok(reader.get(index)?)
            },
            InnerPrecursorReader::DIAPASEF(ref reader) => {
                Ok(reader.get(index)?)
            },
        }
    }
}

#[allow(private_interfaces)]
#[derive(Debug, thiserror::Error)]
pub enum TDFPrecursorReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    DDATDFPrecursorReaderError(#[from] DDATDFPrecursorReaderError),
    #[error("{0}")]
    DIATDFPrecursorReaderError(#[from] DIATDFPrecursorReaderError),
    #[error("Invalid acquistion type for precursor reader: {0}")]
    UnsupportedAcquisition(String),
    #[error("{0}")]
    TDFPathError(#[from] TDFPathError),
    #[error("No path provided for precursor reader")]
    NoPath,
    #[error("No im_converter provided for precursor reader")]
    NoImConverter,
}
