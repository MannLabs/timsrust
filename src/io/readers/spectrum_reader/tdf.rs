mod dda;
mod dia;
mod raw_spectra;

use raw_spectra::{RawSpectrum, RawSpectrumReader, RawSpectrumReaderError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};

use crate::{
    domain_converters::{ConvertableDomain, Tof2MzConverter},
    io::readers::{
        file_readers::sql_reader::{SqlError, SqlReader},
        FrameReader, FrameReaderError, MetadataReader, MetadataReaderError,
        PrecursorReader, PrecursorReaderError,
    },
    ms_data::Spectrum,
    utils::find_extension,
};

use super::{SpectrumReaderConfig, SpectrumReaderTrait};

#[derive(Debug)]
pub struct TDFSpectrumReader {
    path: PathBuf,
    precursor_reader: PrecursorReader,
    mz_reader: Tof2MzConverter,
    raw_spectrum_reader: RawSpectrumReader,
    config: SpectrumReaderConfig,
}

impl TDFSpectrumReader {
    pub fn new(
        path_name: impl AsRef<Path>,
        config: SpectrumReaderConfig,
    ) -> Result<Self, TDFSpectrumReaderError> {
        let frame_reader: FrameReader =
            FrameReader::new(&path_name, config.frame_splitting_params)?;
        let sql_path = find_extension(&path_name, "analysis.tdf").ok_or(
            TDFSpectrumReaderError::FileNotFound("analysis.tdf".to_string()),
        )?;
        let metadata = MetadataReader::new(&sql_path)?;
        let mz_reader: Tof2MzConverter = metadata.mz_converter;
        let tdf_sql_reader = SqlReader::open(&sql_path)?;
        let precursor_reader = PrecursorReader::new(
            &sql_path,
            Some(config.frame_splitting_params),
        )?;
        let acquisition_type = frame_reader.get_acquisition();
        let raw_spectrum_reader = RawSpectrumReader::new(
            &tdf_sql_reader,
            frame_reader,
            acquisition_type,
        )?;
        let reader = Self {
            path: path_name.as_ref().to_path_buf(),
            precursor_reader,
            mz_reader,
            raw_spectrum_reader,
            config,
        };
        Ok(reader)
    }

    pub fn read_single_raw_spectrum(&self, index: usize) -> RawSpectrum {
        let raw_spectrum = self.raw_spectrum_reader.get(index);
        raw_spectrum
            .smooth(self.config.spectrum_processing_params.smoothing_window)
            .centroid(self.config.spectrum_processing_params.centroiding_window)
    }
}

impl SpectrumReaderTrait for TDFSpectrumReader {
    fn get(&self, index: usize) -> Spectrum {
        let raw_spectrum = self.read_single_raw_spectrum(index);
        let spectrum = raw_spectrum.finalize(
            self.precursor_reader.get(index).unwrap(),
            &self.mz_reader,
        );
        spectrum
    }

    fn len(&self) -> usize {
        debug_assert_eq!(
            self.precursor_reader.len(),
            self.raw_spectrum_reader.len()
        );
        self.raw_spectrum_reader.len()
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn calibrate(&mut self) {
        let hits: Vec<(f64, u32)> = (0..self.precursor_reader.len())
            .into_par_iter()
            .map(|index| {
                let spectrum = self.read_single_raw_spectrum(index);
                let precursor = self.precursor_reader.get(index).unwrap();
                let precursor_mz: f64 = precursor.mz;
                let mut result: Vec<(f64, u32)> = vec![];
                for &tof_index in spectrum.tof_indices.iter() {
                    let mz = self.mz_reader.convert(tof_index);
                    if (mz - precursor_mz).abs()
                        < self
                            .config
                            .spectrum_processing_params
                            .calibration_tolerance
                    {
                        let hit = (precursor_mz, tof_index);
                        result.push(hit);
                    }
                }
                result
            })
            .reduce(Vec::new, |mut acc, mut vec| {
                acc.append(&mut vec); // Concatenate vectors
                acc
            });
        if hits.len() >= 2 {
            self.mz_reader = Tof2MzConverter::regress_from_pairs(&hits);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TDFSpectrumReaderError {
    #[error("{0}")]
    SqlError(#[from] SqlError),
    #[error("{0}")]
    PrecursorReaderError(#[from] PrecursorReaderError),
    #[error("{0}")]
    MetadaReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    FrameReaderError(#[from] FrameReaderError),
    #[error("{0}")]
    RawSpectrumReaderError(#[from] RawSpectrumReaderError),
    #[error("{0}")]
    FileNotFound(String),
}
