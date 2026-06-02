mod dda;
mod dia;
mod raw_spectra;

use std::sync::Arc;

use raw_spectra::{RawSpectrum, RawSpectrumReader, RawSpectrumReaderError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use timsrust_core::utils::reader::Reader;
use timsrust_core::{Im, InvertibleConverter, ScanIndex, Spectrum};

use crate::{
    FrameReaderError, FrameWindowSplittingConfiguration, MetadataReaderError,
    TDFPrecursorReader, TDFPrecursorReaderError,
    file_readers::sql_reader::{SqlReader, SqlReaderError},
};

use super::TDFPathLike;

use crate::{TDFPath, TdfFrameReader};

#[derive(Debug)]
pub struct TDFSpectrumReader<ImC> {
    precursor_reader: TDFPrecursorReader<ImC>,
    raw_spectrum_reader: RawSpectrumReader,
    config: SpectrumReaderConfig<ImC>,
}

impl<ImC: InvertibleConverter<ScanIndex, Im> + Send + Sync + std::fmt::Debug>
    TDFSpectrumReader<ImC>
{
    pub fn new(
        path: impl TDFPathLike,
        config: SpectrumReaderConfig<ImC>,
        im_converter: Arc<ImC>,
    ) -> Result<Self, TDFSpectrumReaderError> {
        let frame_reader = TdfFrameReader::new(&path)?;
        let tdf_sql_reader = SqlReader::open(&path)?;
        let precursor_reader = TDFPrecursorReader::build()
            .with_path(&path)
            .with_config(config.clone().frame_splitting_params)
            .with_im_converter(im_converter.clone())
            .finalize()?;
        let acquisition_type = frame_reader.get_acquisition();
        let splitting_strategy = config
            .clone()
            .frame_splitting_params
            .finalize(Some(im_converter));
        let raw_spectrum_reader = RawSpectrumReader::new(
            &tdf_sql_reader,
            frame_reader,
            acquisition_type,
            splitting_strategy,
        )?;
        Ok(Self {
            precursor_reader,
            raw_spectrum_reader,
            config,
        })
    }

    fn read_single_raw_spectrum(
        &self,
        index: usize,
    ) -> Result<RawSpectrum, RawSpectrumReaderError> {
        let raw_spectrum = self
            .raw_spectrum_reader
            .get(index)?
            .smooth(self.config.spectrum_processing_params.smoothing_window)
            .centroid(
                self.config.spectrum_processing_params.centroiding_window,
            );
        Ok(raw_spectrum)
    }

    fn _get(&self, index: usize) -> Result<Spectrum, TDFSpectrumReaderError> {
        let raw_spectrum = self.read_single_raw_spectrum(index)?;
        let spectrum = raw_spectrum
            // .finalize(self.precursor_reader.get(index)?, &self.mz_reader);
            .finalize(self.precursor_reader.get(index)?);
        Ok(spectrum)
    }

    pub fn build() -> SpectrumReaderBuilder<ImC> {
        SpectrumReaderBuilder::default()
    }

    pub fn get_all(&self) -> Vec<Result<Spectrum, TDFSpectrumReaderError>> {
        let mut spectra: Vec<Result<Spectrum, TDFSpectrumReaderError>> = (0
            ..self.len())
            .into_par_iter()
            .map(|index| self._get(index))
            .collect();
        spectra.sort_by_key(|x| match x {
            Ok(spectrum) => match &spectrum.precursor() {
                Some(precursor) => precursor.index(),
                None => spectrum.index(),
            },
            Err(_) => 0,
        });
        spectra
    }

    pub fn len(&self) -> usize {
        debug_assert_eq!(
            self.precursor_reader.len(),
            self.raw_spectrum_reader.len()
        );
        self.raw_spectrum_reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn calibrate(&mut self) {}
}

impl<ImC: InvertibleConverter<ScanIndex, Im> + Send + Sync + std::fmt::Debug>
    timsrust_core::utils::reader::Reader<Spectrum> for TDFSpectrumReader<ImC>
{
    type Error = TDFSpectrumReaderError;
    fn get(&self, index: usize) -> Result<Spectrum, Self::Error> {
        self._get(index)
    }
}

impl<ImC: InvertibleConverter<ScanIndex, Im> + Send + Sync + std::fmt::Debug>
    timsrust_core::utils::reader::IndexedReader<Spectrum>
    for TDFSpectrumReader<ImC>
{
    type Iter = std::ops::Range<usize>;
    fn iter(&self) -> Self::Iter {
        0..self.len()
    }
}

#[allow(private_interfaces)]
#[derive(Debug, thiserror::Error)]
pub enum TDFSpectrumReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    TDFPrecursorReaderError(#[from] TDFPrecursorReaderError),
    #[error("{0}")]
    MetadaReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    FrameReaderError(#[from] FrameReaderError),
    #[error("{0}")]
    RawSpectrumReaderError(#[from] RawSpectrumReaderError),
    #[error("{0}")]
    FileNotFound(String),
    #[error("No precursor")]
    NoPrecursor,
    #[error("No path")]
    NoPath,
    #[error("No im_converter provided")]
    NoImConverter,
}

pub struct SpectrumReaderBuilder<ImC> {
    path: Option<TDFPath>,
    config: SpectrumReaderConfig<ImC>,
    im_converter: Option<Arc<ImC>>,
}

impl<ImC> Default for SpectrumReaderBuilder<ImC> {
    fn default() -> Self {
        Self {
            path: None,
            config: SpectrumReaderConfig::default(),
            im_converter: None,
        }
    }
}

impl<ImC> Clone for SpectrumReaderBuilder<ImC> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            config: self.config.clone(),
            im_converter: self.im_converter.clone(),
        }
    }
}

impl<ImC> SpectrumReaderBuilder<ImC> {
    pub fn with_path(&self, path: impl TDFPathLike) -> Self {
        let path = Some(path.to_timstof_path().unwrap());
        Self {
            path,
            ..self.clone()
        }
    }

    pub fn with_config(&self, config: SpectrumReaderConfig<ImC>) -> Self {
        Self {
            config,
            ..self.clone()
        }
    }

    pub fn with_im_converter(&self, im_converter: Arc<ImC>) -> Self {
        Self {
            im_converter: Some(im_converter),
            ..self.clone()
        }
    }
}

impl<ImC: InvertibleConverter<ScanIndex, Im> + Send + Sync + std::fmt::Debug>
    SpectrumReaderBuilder<ImC>
{
    pub fn finalize(
        self,
    ) -> Result<TDFSpectrumReader<ImC>, TDFSpectrumReaderError> {
        let path = match self.path {
            None => return Err(TDFSpectrumReaderError::NoPath),
            Some(path) => path,
        };
        let im_converter = match self.im_converter {
            None => return Err(TDFSpectrumReaderError::NoImConverter),
            Some(c) => c,
        };
        let mut spectrum_reader =
            TDFSpectrumReader::new(path, self.config.clone(), im_converter)?;
        if self.config.spectrum_processing_params.calibrate {
            spectrum_reader.calibrate();
        }
        Ok(spectrum_reader)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpectrumProcessingParams {
    pub smoothing_window: u32,
    pub centroiding_window: u32,
    pub calibration_tolerance: f64,
    pub calibrate: bool,
}

impl Default for SpectrumProcessingParams {
    fn default() -> Self {
        Self {
            smoothing_window: 1,
            centroiding_window: 1,
            calibration_tolerance: 0.1,
            calibrate: false,
        }
    }
}

#[derive(Debug)]
pub struct SpectrumReaderConfig<ImC> {
    pub spectrum_processing_params: SpectrumProcessingParams,
    pub frame_splitting_params: FrameWindowSplittingConfiguration<ImC>,
}

impl<ImC> Default for SpectrumReaderConfig<ImC> {
    fn default() -> Self {
        Self {
            spectrum_processing_params: SpectrumProcessingParams::default(),
            frame_splitting_params: FrameWindowSplittingConfiguration::default(
            ),
        }
    }
}

impl<ImC> Clone for SpectrumReaderConfig<ImC> {
    fn clone(&self) -> Self {
        Self {
            spectrum_processing_params: self.spectrum_processing_params,
            frame_splitting_params: self.frame_splitting_params.clone(),
        }
    }
}
