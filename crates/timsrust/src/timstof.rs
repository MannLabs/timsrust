use std::path::PathBuf;

use timsrust_core::io::Uri;
use timsrust_minitdf::MiniTDFPath;
use timsrust_parquet_spectra::parquet_path::ParquetSpectrumPath;
use timsrust_tdf::{FrameReaderError, TDFPath, TdfFrameReader};
use timsrust_tsf::TSFPath;

use crate::{
    ImConverter, MzConverter, RtConverter,
    precursor_reader::{
        PrecursorReader, PrecursorReaderBuilder, PrecursorReaderError,
    },
    spectrum_reader::{
        SpectrumReader, SpectrumReaderBuilder, SpectrumReaderError,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum TimsTofFileType {
    MiniTdf(MiniTDFPath),
    Tdf(TDFPath),
    Parquet(ParquetSpectrumPath),
    Tsf(TSFPath),
    #[cfg(feature = "patched")]
    Patched(timsrust_patched::PatchedTimsTofPath),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimsTofPath {
    uri: Uri,
    file_type: TimsTofFileType,
}

impl TimsTofPath {
    pub fn new(path: impl AsRef<str>) -> Result<Self, TimsTofPathError> {
        let uri = Uri::from(path.as_ref()).local_representation();
        for path in [uri.as_ref(), path.as_ref()] {
            #[cfg(feature = "patched")]
            if let Ok(patched_path) =
                timsrust_patched::PatchedTimsTofPath::new(path)
            {
                return Ok(Self {
                    uri: uri.clone(),
                    file_type: TimsTofFileType::Patched(patched_path),
                });
            }
            if let Ok(tdf) = TDFPath::new(path) {
                return Ok(Self {
                    uri: tdf.uri().clone(),
                    file_type: TimsTofFileType::Tdf(tdf),
                });
            }
            if let Ok(tsf) = TSFPath::new(path) {
                return Ok(Self {
                    uri: tsf.uri().clone(),
                    file_type: TimsTofFileType::Tsf(tsf),
                });
            }
            if let Ok(minitdf) = MiniTDFPath::new(path) {
                return Ok(Self {
                    uri: minitdf.uri().clone(),
                    file_type: TimsTofFileType::MiniTdf(minitdf),
                });
            }

            if let Ok(parquet) = ParquetSpectrumPath::new(path) {
                return Ok(Self {
                    uri: parquet.uri().clone(),
                    file_type: TimsTofFileType::Parquet(parquet),
                });
            }
        }
        Err(TimsTofPathError::UnknownType(PathBuf::from(path.as_ref())))
    }

    pub(crate) fn file_type(&self) -> &TimsTofFileType {
        &self.file_type
    }

    /// Create a [`SpectrumReader`] for this path.
    pub fn spectrum_reader(
        &self,
    ) -> Result<SpectrumReader, SpectrumReaderError> {
        SpectrumReaderBuilder::default().with_path(self).finalize()
    }

    /// Create a [`PrecursorReader`] for this path.
    pub fn precursor_reader(
        &self,
    ) -> Result<PrecursorReader, PrecursorReaderError> {
        PrecursorReaderBuilder::default().with_path(self).finalize()
    }

    /// Create a [`TdfFrameReader`] for this path.
    ///
    /// Only supported for `.d` (TDF) files; returns
    /// [`TimsTofFrameReaderError::NotSupported`] for all other file types.
    pub fn frame_reader(
        &self,
    ) -> Result<TdfFrameReader, TimsTofFrameReaderError> {
        match &self.file_type {
            TimsTofFileType::Tdf(tdf_path) => {
                Ok(TdfFrameReader::new(tdf_path)?)
            },
            _ => Err(TimsTofFrameReaderError::NotSupported),
        }
    }

    /// Create an [`MzConverter`] (TOF index → m/z) for this path.
    pub fn mz_converter(&self) -> Option<MzConverter> {
        MzConverter::new(self)
    }

    /// Create an [`ImConverter`] (scan index → ion mobility) for this path.
    pub fn im_converter(&self) -> Option<ImConverter> {
        ImConverter::new(self)
    }

    /// Create an [`RtConverter`] (frame index → retention time) for this path.
    pub fn rt_converter(&self) -> Option<RtConverter> {
        RtConverter::new(self)
    }
}

impl AsRef<str> for TimsTofPath {
    fn as_ref(&self) -> &str {
        self.uri.as_ref()
    }
}

pub trait TimsTofPathLike: AsRef<str> {
    fn to_timstof_path(&self) -> Result<TimsTofPath, TimsTofPathError>;
}

impl<T: AsRef<str>> TimsTofPathLike for T {
    fn to_timstof_path(&self) -> Result<TimsTofPath, TimsTofPathError> {
        TimsTofPath::new(self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TimsTofPathError {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("No valid type found for {0}")]
    UnknownType(PathBuf),
}

/// Error returned by [`TimsTofPath::frame_reader`].
#[derive(Debug, thiserror::Error)]
pub enum TimsTofFrameReaderError {
    #[error("{0}")]
    FrameReaderError(#[from] FrameReaderError),
    #[error("Frame reading is not supported for this file type")]
    NotSupported,
}
