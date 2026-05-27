use std::sync::Arc;

use rayon::prelude::*;
#[cfg(feature = "bps")]
use timsrust_core::AcquisitionType;
use timsrust_core::{Mz, Spectrum};
use timsrust_minitdf::{MiniTDFError, MiniTDFSpectrumReader};
#[cfg(feature = "bps")]
use timsrust_tdf::{FrameInfoReader, Metadata, TdfIonReader};
use timsrust_tdf::{
    SpectrumReaderConfig, TDFSpectrumReader, TDFSpectrumReaderError,
};

use crate::{
    ImConverter, TimsTofPath, TimsTofPathLike, converters::MzConverter,
    timstof::TimsTofFileType,
};
use timsrust_core::utils::reader::{ParIterableReader, Reader};

#[allow(clippy::large_enum_variant)]
enum Inner {
    #[cfg(feature = "bps")]
    Centroider(
        timsrust_bruker::centroid::spectrum_reader::SpectrumReader<
            TdfIonReader,
            FrameInfoReader,
            ImConverter,
            MzConverter,
        >,
    ),
    Tdf(TDFSpectrumReader<ImConverter>),
    MiniTdf(MiniTDFSpectrumReader),
    #[cfg(feature = "bps")]
    ParquetSpectra(
        timsrust_bruker::parquet_spectra::spectrum_reader::ParquetSpectrumReader,
    ),
}

impl Inner {
    fn get(
        &self,
        index: usize,
    ) -> Result<timsrust_core::Spectrum, SpectrumReaderError> {
        match self {
            #[cfg(feature = "bps")]
            Inner::Centroider(_) => {
                Err(SpectrumReaderError::CentroiderNotSupported)
            },
            Inner::Tdf(reader) => Ok(reader.get(index)?),
            Inner::MiniTdf(reader) => Ok(reader.get(index)?),
            #[cfg(feature = "bps")]
            Inner::ParquetSpectra(reader) => Ok(reader.get(index)?),
        }
    }

    fn len(&self) -> usize {
        match self {
            Inner::Tdf(reader) => reader.len(),
            Inner::MiniTdf(reader) => reader.len(),
            #[cfg(feature = "bps")]
            Inner::Centroider(reader) => reader.len(),
            #[cfg(feature = "bps")]
            Inner::ParquetSpectra(reader) => reader.len(),
        }
    }

    fn calibrate(&mut self) {
        if let Inner::Tdf(reader) = self {
            reader.calibrate()
        }
    }

    fn par_iter(
        &self,
    ) -> impl ParallelIterator<Item = timsrust_core::Spectrum> + '_ {
        match self {
            #[cfg(feature = "bps")]
            Inner::Centroider(reader) => A::Centroider(reader),
            Inner::Tdf(reader) => A::Tdf(reader),
            Inner::MiniTdf(reader) => A::MiniTdf(reader),
            #[cfg(feature = "bps")]
            Inner::ParquetSpectra(reader) => A::ParquetSpectra(reader),
        }
    }
}

enum A<'a> {
    #[cfg(feature = "bps")]
    Centroider(
        &'a timsrust_bruker::centroid::spectrum_reader::SpectrumReader<
            TdfIonReader,
            FrameInfoReader,
            ImConverter,
            MzConverter,
        >,
    ),
    Tdf(&'a TDFSpectrumReader<ImConverter>),
    MiniTdf(&'a MiniTDFSpectrumReader),
    #[cfg(feature = "bps")]
    ParquetSpectra(
        &'a timsrust_bruker::parquet_spectra::spectrum_reader::ParquetSpectrumReader,
    ),
}

impl<'a> ParallelIterator for A<'a> {
    type Item = timsrust_core::Spectrum;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        match self {
            #[cfg(feature = "bps")]
            Self::Centroider(reader) => reader
                .par_iter()
                .filter_map(|s| s.ok())
                .drive_unindexed(consumer),
            Self::Tdf(reader) => rayon::iter::ParallelIterator::filter_map(
                reader.par_iter(),
                |s| s.ok(),
            )
            .drive_unindexed(consumer),
            Self::MiniTdf(reader) => rayon::iter::ParallelIterator::filter_map(
                reader.par_iter(),
                |s| s.ok(),
            )
            .drive_unindexed(consumer),
            #[cfg(feature = "bps")]
            Self::ParquetSpectra(reader) => {
                rayon::iter::ParallelIterator::filter_map(
                    reader.par_iter(),
                    |s| s.ok(),
                )
                .drive_unindexed(consumer)
            },
        }
    }
}

impl<'a> IntoParallelIterator for &'a A<'a> {
    type Item = Result<Spectrum, SpectrumReaderError>;
    type Iter = rayon::vec::IntoIter<Result<Spectrum, SpectrumReaderError>>;

    fn into_par_iter(self) -> Self::Iter {
        self.par_iter()
    }
}

pub struct SpectrumReader {
    spectrum_reader: Inner,
    mz_converter: Arc<MzConverter>,
}

impl SpectrumReader {
    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Self, SpectrumReaderError> {
        Self::build().with_path(path).finalize()
    }

    pub fn build() -> SpectrumReaderBuilder {
        SpectrumReaderBuilder::default()
    }

    pub fn get(
        &self,
        index: usize,
    ) -> Result<Spectrum<Mz>, SpectrumReaderError> {
        let spectrum = self.spectrum_reader.get(index)?;
        Ok(spectrum.convert_to(self.mz_converter.as_ref()))
    }

    pub fn len(&self) -> usize {
        self.spectrum_reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_all(&self) -> Vec<Result<Spectrum<Mz>, SpectrumReaderError>> {
        let mut spectra: Vec<Result<Spectrum<Mz>, SpectrumReaderError>> = (0
            ..self.len())
            .into_par_iter()
            .map(|index| self.get(index))
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

    fn calibrate(&mut self) {
        self.spectrum_reader.calibrate();
    }

    pub fn par_iter(
        &self,
    ) -> impl ParallelIterator<Item = Result<Spectrum<Mz>, SpectrumReaderError>> + '_
    {
        self.spectrum_reader
            .par_iter()
            .map(|spectrum| Ok(spectrum.convert_to(self.mz_converter.as_ref())))
    }

    // pub fn _into_par_iter(
    //     &self,
    // ) -> impl IntoParallelIterator<Item = Result<Spectrum, SpectrumReaderError>> + '_
    // {
    //     self.spectrum_reader
    //         .into_par_iter()
    //         .map(|spectrum| Ok(spectrum))
    // }
}

// impl ParallelIterator for SpectrumReader {
//     type Item = Result<Spectrum, SpectrumReaderError>;

//     fn drive_unindexed<C>(self, consumer: C) -> C::Result
//     where
//         C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
//     {
//         self._par_iter().drive_unindexed(consumer)
//     }
// }

// impl IntoParallelIterator for SpectrumReader {
//     type Item = Result<Spectrum, SpectrumReaderError>;
//     type Iter = rayon::vec::IntoIter<Result<Spectrum, SpectrumReaderError>>;

//     fn into_par_iter(self) -> Self::Iter {
//         self.par_iter()
//     }
// }

#[derive(Debug, thiserror::Error)]
pub enum SpectrumReaderError {
    #[error("{0}")]
    TDFSpectrumReaderError(#[from] TDFSpectrumReaderError),
    #[error("{0}")]
    MiniTDFSpectrumReaderError(#[from] MiniTDFError),
    #[cfg(feature = "bps")]
    #[error("{0}")]
    ParquetSpectrumReaderError(
        #[from]
        timsrust_bruker::parquet_spectra::spectrum_reader::ParquetSpectrumReaderError,
    ),
    #[error("No path provided")]
    NoPath,
    #[error("Centroider is not supported")]
    CentroiderNotSupported,
}

#[derive(Debug, Default, Clone)]
pub struct SpectrumReaderBuilder {
    path: Option<TimsTofPath>,
    config: SpectrumReaderConfig<ImConverter>,
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

    pub fn with_config(
        &self,
        config: SpectrumReaderConfig<ImConverter>,
    ) -> Self {
        Self {
            config,
            ..self.clone()
        }
    }

    pub fn finalize(self) -> Result<SpectrumReader, SpectrumReaderError> {
        let path = match self.path {
            None => return Err(SpectrumReaderError::NoPath),
            Some(path) => path,
        };
        let spectrum_reader = match path.file_type() {
            TimsTofFileType::Tdf(tdf_path) => {
                #[cfg(feature = "bps")]
                if Metadata::new(tdf_path.as_ref()).unwrap().acquisition_type()
                    == AcquisitionType::DIAPASEF
                {
                    use timsrust_tdf::TdfFrameReader;

                    let im_converter = ImConverter::new(&path).unwrap();
                    let mz_converter = MzConverter::new(&path).unwrap();
                    let frame_reader =
                        TdfFrameReader::new(tdf_path.as_ref())
                            .unwrap()
                            .into_inner();
                    let spectrum_reader = Inner::Centroider(
                        timsrust_bruker::centroid::spectrum_reader::SpectrumReader::new(
                            frame_reader,
                            0.5,
                            // 15,
                            // 5,
                            2.0,
                            5,
                            true,
                            im_converter,
                            mz_converter,
                        )
                        .unwrap(),
                    );
                    let mz_converter =
                        Arc::new(MzConverter::new(&path).unwrap());
                    return Ok(SpectrumReader {
                        spectrum_reader,
                        mz_converter,
                    });
                }
                let im_converter =
                    Arc::new(ImConverter::new(&path).unwrap());
                Inner::Tdf(TDFSpectrumReader::new(
                    tdf_path,
                    self.config.clone(),
                    im_converter,
                )?)
            },
            // TimsTofFileType::Tdf(tdf_path) => {
            //     Inner::Tdf(TDFSpectrumReader::new(tdf_path, self.config.clone())?)
            // },
            TimsTofFileType::MiniTdf(mini_path) => {
                Inner::MiniTdf(mini_path.spectrum_reader()?)
            },
            #[cfg(feature = "bps")]
            TimsTofFileType::Parquet(parquet_path) => {
                Inner::ParquetSpectra(
                    timsrust_bruker::parquet_spectra::spectrum_reader::ParquetSpectrumReader::new(
                        parquet_path.precursor_path(),
                        parquet_path.fragment_path(),
                    ),
                )
            },
        };
        let mz_converter = Arc::new(MzConverter::new(&path).unwrap());
        let mut reader = SpectrumReader {
            spectrum_reader,
            mz_converter,
        };
        if self.config.spectrum_processing_params.calibrate {
            reader.calibrate();
        }
        Ok(reader)
    }
}
