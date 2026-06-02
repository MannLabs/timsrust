use crate::{
    MiniTDFPathError, MiniTDFPrecursorReaderError, MiniTDFSpectrumReaderError,
};

#[derive(Debug, thiserror::Error)]
pub enum MiniTDFError {
    #[error("{0}")]
    #[allow(private_interfaces)]
    MiniTDFPrecursorReaderError(#[from] MiniTDFPrecursorReaderError),
    #[error("{0}")]
    #[allow(private_interfaces)]
    MiniTDFSpectrumReaderError(#[from] MiniTDFSpectrumReaderError),
    #[error("{0}")]
    MiniTDFPathError(#[from] MiniTDFPathError),
}

pub type MiniTDFResult<T> = Result<T, MiniTDFError>;
