use std::path::{Path, PathBuf};

use crate::ms_data::Spectrum;

use super::SpectrumReaderTrait;

#[derive(Debug)]
pub struct MiniTDFSpectrumReader {
    path: PathBuf,
}

impl MiniTDFSpectrumReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl SpectrumReaderTrait for MiniTDFSpectrumReader {
    fn get(&self, index: usize) -> Spectrum {
        Spectrum::default()
    }

    fn len(&self) -> usize {
        0 //TODO
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}
