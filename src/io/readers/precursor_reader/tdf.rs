mod dda;
mod dia;

use std::path::Path;

use dda::{DDATDFPrecursorReader, DDATDFPrecursorReaderError};
use dia::{DIATDFPrecursorReader, DIATDFPrecursorReaderError};

use crate::{
    io::readers::{
        file_readers::sql_reader::{SqlError, SqlReader},
        FrameWindowSplittingStrategy,
    },
    ms_data::{AcquisitionType, Precursor},
};

use super::PrecursorReaderTrait;

pub struct TDFPrecursorReader {
    precursor_reader: Box<dyn PrecursorReaderTrait>,
}

impl TDFPrecursorReader {
    pub fn new(
        path: impl AsRef<Path>,
        splitting_strategy: Option<FrameWindowSplittingStrategy>,
    ) -> Result<Self, TDFPrecursorReaderError> {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(sql_path)?;
        let sql_frames: Vec<u8> =
            tdf_sql_reader.read_column_from_table("ScanMode", "Frames")?;
        let acquisition_type = if sql_frames.iter().any(|&x| x == 8) {
            AcquisitionType::DDAPASEF
        } else if sql_frames.iter().any(|&x| x == 9) {
            AcquisitionType::DIAPASEF
        } else {
            AcquisitionType::Unknown
        };
        let precursor_reader: Box<dyn PrecursorReaderTrait> =
            match (acquisition_type, splitting_strategy) {
                (AcquisitionType::DDAPASEF, None) => {
                    Box::new(DDATDFPrecursorReader::new(path)?)
                },
                (
                    AcquisitionType::DDAPASEF,
                    Some(FrameWindowSplittingStrategy::None),
                ) => {
                    // Not 100% sure when this happens ...
                    // By this I mean generating a Some(None)
                    // ./tests/frame_readers.rs:60:25 generates it.
                    // JSPP - 2024-Jul-16
                    Box::new(DDATDFPrecursorReader::new(path)?)
                },
                (AcquisitionType::DIAPASEF, Some(splitting_strat)) => {
                    Box::new(DIATDFPrecursorReader::new(path, splitting_strat)?)
                },
                (AcquisitionType::DIAPASEF, None) => {
                    Box::new(DIATDFPrecursorReader::new(
                        path,
                        FrameWindowSplittingStrategy::None,
                    )?)
                },
                (acq_type, acq_config) => {
                    return Err(TDFPrecursorReaderError::UnknownPrecursorType(
                        format!("{:?} + {:?}", acquisition_type, acq_config),
                    ))
                },
            };
        let reader = Self { precursor_reader };
        Ok(reader)
    }
}

impl PrecursorReaderTrait for TDFPrecursorReader {
    fn get(&self, index: usize) -> Option<Precursor> {
        self.precursor_reader.get(index)
    }

    fn len(&self) -> usize {
        self.precursor_reader.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TDFPrecursorReaderError {
    #[error("{0}")]
    SqlError(#[from] SqlError),
    #[error("{0}")]
    DDATDFPrecursorReaderError(#[from] DDATDFPrecursorReaderError),
    #[error("{0}")]
    DIATDFPrecursorReaderError(#[from] DIATDFPrecursorReaderError),
    #[error("Invalid acquistion type for precursor reader: {0}")]
    UnknownPrecursorType(String),
}
