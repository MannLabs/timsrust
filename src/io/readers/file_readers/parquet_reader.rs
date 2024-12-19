pub mod precursors;

use std::{fs::File, io, str::FromStr};

use parquet::file::reader::{FileReader, SerializedFileReader};

use crate::readers::TimsTofPathError;

pub trait ReadableParquetTable {
    fn update_from_parquet_file(&mut self, key: &str, value: String);

    fn parse_default_field<T: FromStr + Default>(field: String) -> T {
        field.parse().unwrap_or_default()
    }

    fn from_parquet_file(
        path: impl crate::readers::TimsTofPathLike,
    ) -> Result<Vec<Self>, ParquetReaderError>
    where
        Self: Sized + Default,
    {
        let path = path.to_timstof_path()?;
        let file: File = File::open(path.ms2_parquet()?)?;
        let reader: SerializedFileReader<File> =
            SerializedFileReader::new(file)?;
        reader
            .get_row_iter(None)?
            .map(|record| {
                let mut result = Self::default();
                for (name, field) in record?.get_column_iter() {
                    result.update_from_parquet_file(
                        name.to_string().as_str(),
                        field.to_string(),
                    );
                }
                Ok(result)
            })
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParquetReaderError {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("Cannot iterate over row {0}")]
    ParquetError(#[from] parquet::errors::ParquetError),
    #[error("{0}")]
    TimsTofPathError(#[from] TimsTofPathError),
}
