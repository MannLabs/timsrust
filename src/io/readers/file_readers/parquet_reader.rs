pub mod precursors;

use parquet::{
    file::reader::{FileReader, SerializedFileReader},
    record::Field,
};
use std::{fs::File, io, path::Path};

pub trait ReadableParquetTable {
    fn update_from_parquet_file(&mut self, name: &String, field: &Field);

    fn from_parquet_file(
        file_name: impl AsRef<Path>,
    ) -> Result<Vec<Self>, ParquetError>
    where
        Self: Sized + Default,
    {
        let file: File = File::open(file_name)?;
        let reader: SerializedFileReader<File> =
            SerializedFileReader::new(file)?;
        let results: Vec<Self> = reader
            .get_row_iter(None)?
            .map(|record| {
                let mut result = Self::default();
                for (name, field) in record.get_column_iter() {
                    result.update_from_parquet_file(name, field);
                }
                result
            })
            .collect();
        Ok(results)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParquetError {
    #[error("Cannot read file {0}")]
    IO(#[from] io::Error),
    #[error("Cannot iterate over row {0}")]
    ParquetIO(#[from] parquet::errors::ParquetError),
}
