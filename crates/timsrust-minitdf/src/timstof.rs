use timsrust_core::io::Uri;

use crate::{
    MiniTDFError, precursors::MiniTDFPrecursorReader,
    spectrum::MiniTDFSpectrumReader,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MiniTDFPath {
    uri: Uri,
    bin: Uri,
    parquet: Uri,
}

impl MiniTDFPath {
    pub fn new(path: impl AsRef<str>) -> Result<Self, MiniTDFPathError> {
        let uri = Uri::from(path.as_ref());
        let bin_uri = uri.join("ms2spectrum.bin");
        let parquet_uri = uri.join("ms2spectrum.parquet");
        if bin_uri.probe_is_file() && parquet_uri.probe_is_file() {
            return Ok(Self {
                uri,
                bin: bin_uri,
                parquet: parquet_uri,
            });
        }
        match uri.parent() {
            Some(parent) => Self::new(parent.as_ref())
                .map_err(|_| MiniTDFPathError::UnknownType(uri.to_string())),
            None => Err(MiniTDFPathError::UnknownType(uri.to_string())),
        }
    }

    pub fn ms2_bin(&self) -> &Uri {
        &self.bin
    }

    pub fn ms2_parquet(&self) -> &Uri {
        &self.parquet
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn spectrum_reader(
        &self,
    ) -> Result<MiniTDFSpectrumReader, MiniTDFError> {
        MiniTDFSpectrumReader::new(self)
    }

    pub fn precursor_reader(
        &self,
    ) -> Result<MiniTDFPrecursorReader, MiniTDFError> {
        MiniTDFPrecursorReader::new(self)
    }
}

impl AsRef<str> for MiniTDFPath {
    fn as_ref(&self) -> &str {
        self.uri.as_ref()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MiniTDFPathError {
    #[error("No valid type found for {0}")]
    UnknownType(String),
}
