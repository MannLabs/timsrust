mod dda;
mod dia;

use dda::{DDATDFPrecursorReader, DDATDFPrecursorReaderError};
use dia::{DIATDFPrecursorReader, DIATDFPrecursorReaderError};

use crate::{
    io::readers::{
        file_readers::sql_reader::{SqlReader, SqlReaderError},
        FrameWindowSplittingConfiguration,
    },
    ms_data::{AcquisitionType, Precursor},
    readers::TimsTofPathLike,
};

use super::PrecursorReaderTrait;

pub struct TDFPrecursorReader {
    precursor_reader: Box<dyn PrecursorReaderTrait>,
}

impl TDFPrecursorReader {
    pub fn new(
        path: impl TimsTofPathLike,
        splitting_strategy: FrameWindowSplittingConfiguration,
    ) -> Result<Self, TDFPrecursorReaderError> {
        let tdf_sql_reader = SqlReader::open(&path)?;
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
            match acquisition_type {
                AcquisitionType::DDAPASEF => {
                    Box::new(DDATDFPrecursorReader::new(&path)?)
                },
                AcquisitionType::DIAPASEF => Box::new(
                    DIATDFPrecursorReader::new(path, splitting_strategy)?,
                ),
                acquisition_type => {
                    return Err(
                        TDFPrecursorReaderError::UnsupportedAcquisition(
                            format!("{:?}", acquisition_type),
                        ),
                    )
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
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    DDATDFPrecursorReaderError(#[from] DDATDFPrecursorReaderError),
    #[error("{0}")]
    DIATDFPrecursorReaderError(#[from] DIATDFPrecursorReaderError),
    #[error("Invalid acquistion type for precursor reader: {0}")]
    UnsupportedAcquisition(String),
}
