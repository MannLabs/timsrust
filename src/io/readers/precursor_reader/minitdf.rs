use crate::{
    io::readers::file_readers::parquet_reader::{
        precursors::ParquetPrecursor, ParquetReaderError, ReadableParquetTable,
    },
    ms_data::Precursor,
    readers::TimsTofPathLike,
};

use super::PrecursorReaderTrait;

#[derive(Debug)]
pub struct MiniTDFPrecursorReader {
    parquet_precursors: Vec<ParquetPrecursor>,
}

impl MiniTDFPrecursorReader {
    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Self, MiniTDFPrecursorReaderError> {
        let parquet_precursors = ParquetPrecursor::from_parquet_file(path)?;
        let reader = Self { parquet_precursors };
        Ok(reader)
    }
}

impl PrecursorReaderTrait for MiniTDFPrecursorReader {
    fn get(&self, index: usize) -> Option<Precursor> {
        let parquet_precursor = &self.parquet_precursors.get(index)?;
        let precursor = Precursor {
            mz: parquet_precursor.mz,
            rt: parquet_precursor.rt,
            im: parquet_precursor.im,
            charge: Some(parquet_precursor.charge),
            intensity: Some(parquet_precursor.intensity),
            index: parquet_precursor.index,
            frame_index: parquet_precursor.frame_index,
        };
        Some(precursor)
    }

    fn len(&self) -> usize {
        self.parquet_precursors.len()
    }
}

#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct MiniTDFPrecursorReaderError(#[from] ParquetReaderError);
