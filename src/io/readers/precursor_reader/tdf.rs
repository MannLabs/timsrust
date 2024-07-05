mod dda;
mod dia;

use std::path::{Path, PathBuf};

use dda::DDATDFPrecursorReader;
use dia::DIATDFPrecursorReader;

use crate::{
    io::readers::file_readers::sql_reader::SqlReader,
    ms_data::{AcquisitionType, Precursor},
};

use super::PrecursorReaderTrait;

pub struct TDFPrecursorReader {
    precursor_reader: Box<dyn PrecursorReaderTrait>,
}

impl TDFPrecursorReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(sql_path).unwrap();
        let sql_frames: Vec<u8> = tdf_sql_reader
            .read_column_from_table("ScanMode", "Frames")
            .unwrap();
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
                    Box::new(DDATDFPrecursorReader::new(path))
                },
                AcquisitionType::DIAPASEF => {
                    Box::new(DIATDFPrecursorReader::new(path))
                },
                _ => panic!(),
            };
        Self { precursor_reader }
    }
}

impl PrecursorReaderTrait for TDFPrecursorReader {
    fn get(&self, index: usize) -> Precursor {
        self.precursor_reader.get(index)
    }

    fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    fn get_path(&self) -> PathBuf {
        self.precursor_reader.get_path()
    }
}
