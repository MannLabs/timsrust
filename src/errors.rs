#[cfg(feature = "tdf")]
use crate::io::readers::{
    FrameReaderError, MetadataReaderError, QuadrupoleSettingsReaderError,
};
use crate::{io::readers::PrecursorReaderError, readers::SpectrumReaderError};

/// An error that is produced by timsrust (uses [thiserror]).
#[derive(thiserror::Error, Debug)]
pub enum TimsRustError {
    #[cfg(feature = "tdf")]
    #[error("{0}")]
    FrameReaderError(#[from] FrameReaderError),
    #[error("{0}")]
    SpectrumReaderError(#[from] SpectrumReaderError),
    #[cfg(feature = "tdf")]
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    PrecursorReaderError(#[from] PrecursorReaderError),
    #[cfg(feature = "tdf")]
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
}
