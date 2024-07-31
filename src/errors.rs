use crate::io::readers::{
    FrameReaderError, MetadataReaderError, PrecursorReaderError,
    QuadrupoleSettingsReaderError, SpectrumReaderError,
};

/// An error that is produced by timsrust (uses [thiserror]).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    FrameReaderError(#[from] FrameReaderError),
    #[error("{0}")]
    SpectrumReaderError(#[from] SpectrumReaderError),
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    PrecursorReaderError(#[from] PrecursorReaderError),
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
}
