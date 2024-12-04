#[cfg(feature = "minitdf")]
use super::minitdf::MiniTDFSpectrumReaderError;
#[cfg(feature = "tdf")]
use super::tdf::TDFSpectrumReaderError;

#[derive(Debug, thiserror::Error)]
pub enum SpectrumReaderError {
    #[cfg(feature = "minitdf")]
    #[error("{0}")]
    MiniTDFSpectrumReaderError(#[from] MiniTDFSpectrumReaderError),
    #[cfg(feature = "tdf")]
    #[error("{0}")]
    TDFSpectrumReaderError(#[from] TDFSpectrumReaderError),
    #[error("No path provided")]
    NoPath,
}
