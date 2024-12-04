use crate::readers::{TimsTofFileType, TimsTofPath, TimsTofPathLike};

use super::{
    errors::SpectrumReaderError, SpectrumReader, SpectrumReaderConfig,
    SpectrumReaderTrait,
};

#[cfg(feature = "minitdf")]
use super::minitdf::MiniTDFSpectrumReader;
#[cfg(feature = "tdf")]
use super::tdf::TDFSpectrumReader;

#[derive(Debug, Default, Clone)]
pub struct SpectrumReaderBuilder {
    path: Option<TimsTofPath>,
    config: SpectrumReaderConfig,
}

impl SpectrumReaderBuilder {
    pub fn with_path(&self, path: impl TimsTofPathLike) -> Self {
        // TODO
        let path = Some(path.to_timstof_path().unwrap());
        Self {
            path,
            ..self.clone()
        }
    }

    pub fn with_config(&self, config: SpectrumReaderConfig) -> Self {
        Self {
            config: config,
            ..self.clone()
        }
    }

    pub fn finalize(self) -> Result<SpectrumReader, SpectrumReaderError> {
        let path = match self.path {
            None => return Err(SpectrumReaderError::NoPath),
            Some(path) => path,
        };
        let spectrum_reader: Box<dyn SpectrumReaderTrait> =
            match path.file_type() {
                #[cfg(feature = "minitdf")]
                TimsTofFileType::MiniTDF => {
                    Box::new(MiniTDFSpectrumReader::new(path)?)
                },
                #[cfg(feature = "tdf")]
                TimsTofFileType::TDF => {
                    Box::new(TDFSpectrumReader::new(path, self.config)?)
                },
            };
        let mut reader = SpectrumReader { spectrum_reader };
        if self.config.spectrum_processing_params.calibrate {
            reader.calibrate();
        }
        Ok(reader)
    }
}
