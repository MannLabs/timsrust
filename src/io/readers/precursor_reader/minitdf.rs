use std::path::{Path, PathBuf};

use crate::{
    io::readers::file_readers::parquet_reader::{
        precursors::ParquetPrecursor, ReadableParquetTable,
    },
    ms_data::Precursor,
};

use super::PrecursorReaderTrait;

#[derive(Debug)]
pub struct MiniTDFPrecursorReader {
    path: PathBuf,
    parquet_precursors: Vec<ParquetPrecursor>,
}

impl MiniTDFPrecursorReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let parquet_precursors =
            ParquetPrecursor::from_parquet_file(&path).unwrap();
        Self {
            path: path.as_ref().to_path_buf(),
            parquet_precursors,
        }
    }
}

impl PrecursorReaderTrait for MiniTDFPrecursorReader {
    fn get(&self, index: usize) -> Precursor {
        let x = &self.parquet_precursors[index];
        Precursor {
            mz: x.mz,
            rt: x.rt,
            im: x.im,
            charge: x.charge,
            intensity: x.intensity,
            index: x.index,
            frame_index: x.frame_index,
            collision_energy: x.collision_energy,
        }
    }

    fn len(&self) -> usize {
        self.parquet_precursors.len()
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}
