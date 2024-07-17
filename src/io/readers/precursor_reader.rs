mod minitdf;
mod tdf;

use core::fmt;
use std::path::{Path, PathBuf};

use minitdf::MiniTDFPrecursorReader;
use tdf::TDFPrecursorReader;

use crate::ms_data::Precursor;

use super::FrameWindowSplittingStrategy;

pub struct PrecursorReader {
    precursor_reader: Box<dyn PrecursorReaderTrait>,
}

impl fmt::Debug for PrecursorReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrecursorReader {{ /* fields omitted */ }}")
    }
}

impl PrecursorReader {
    pub fn new(
        path: impl AsRef<Path>,
        config: Option<FrameWindowSplittingStrategy>,
    ) -> Self {
        let tmp = path.as_ref().extension().and_then(|e| e.to_str());
        let precursor_reader: Box<dyn PrecursorReaderTrait> =
            match (tmp, config) {
                (Some("parquet"), None) => {
                    Box::new(MiniTDFPrecursorReader::new(path))
                },
                (Some("tdf"), strat) => {
                    Box::new(TDFPrecursorReader::new(path, strat))
                },
                _ => panic!(),
            };
        Self { precursor_reader }
    }

    pub fn get(&self, index: usize) -> Precursor {
        self.precursor_reader.get(index)
    }

    pub fn get_path(&self) -> PathBuf {
        self.precursor_reader.get_path()
    }

    pub fn len(&self) -> usize {
        self.precursor_reader.len()
    }
}

trait PrecursorReaderTrait: Sync {
    fn get(&self, index: usize) -> Precursor;
    fn get_path(&self) -> PathBuf;
    fn len(&self) -> usize;
}
