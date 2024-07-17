mod dda;
mod dia;

use std::path::{Path, PathBuf};

use dda::DDATDFPrecursorReader;
use dia::DIATDFPrecursorReader;

use crate::{
    io::readers::{
        file_readers::sql_reader::SqlReader, FrameWindowSplittingStrategy,
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
    ) -> Self {
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
            match (acquisition_type, splitting_strategy) {
                (AcquisitionType::DDAPASEF, None) => {
                    Box::new(DDATDFPrecursorReader::new(path))
                },
                (
                    AcquisitionType::DDAPASEF,
                    Some(FrameWindowSplittingStrategy::None),
                ) => {
                    // Not 100% sure when this happens ...
                    // By this I mean generating a Some(None)
                    // ./tests/frame_readers.rs:60:25 generates it.
                    // JSPP - 2024-Jul-16
                    Box::new(DDATDFPrecursorReader::new(path))
                },
                (AcquisitionType::DIAPASEF, Some(splitting_strat)) => {
                    Box::new(DIATDFPrecursorReader::new(path, splitting_strat))
                },
                (AcquisitionType::DIAPASEF, None) => {
                    Box::new(DIATDFPrecursorReader::new(
                        path,
                        FrameWindowSplittingStrategy::None,
                    ))
                },
                _ => panic!(
                    "No idea how to handle {:?} - {:?}",
                    acquisition_type, splitting_strategy
                ),
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
