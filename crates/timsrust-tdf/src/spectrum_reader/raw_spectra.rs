use timsrust_core::utils::vec::{
    filter_with_mask, find_sparse_local_maxima_mask,
};
use timsrust_core::{
    AcquisitionType, Im, InvertibleConverter, Mz, Precursor, ScanIndex,
    Spectrum, TofIndex,
};

use crate::{
    TdfFrameReader, file_readers::sql_reader::SqlReader,
    quad_settings_reader::FrameWindowSplittingStrategy,
};

use super::{
    dda::{DDARawSpectrumReader, DDARawSpectrumReaderError},
    dia::{DIARawSpectrumReader, DIARawSpectrumReaderError},
};

#[derive(Debug, PartialEq, Default, Clone)]
pub(crate) struct RawSpectrum {
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u64>,
    pub index: usize,
    pub collision_energy: f64,
    pub isolation_mz: f64,
    pub isolation_width: f64,
}

impl RawSpectrum {
    pub(crate) fn smooth(mut self, window: u32) -> Self {
        let mut smooth_intensities: Vec<u64> = self.intensities.clone();
        for (current_index, current_tof) in self.tof_indices.iter().enumerate()
        {
            let current_intensity: u64 = self.intensities[current_index];
            for (_next_index, next_tof) in
                self.tof_indices[current_index + 1..].iter().enumerate()
            {
                let next_index: usize = _next_index + current_index + 1;
                let next_intensity: u64 = self.intensities[next_index];
                if (next_tof - current_tof) <= window {
                    smooth_intensities[current_index] += next_intensity;
                    smooth_intensities[next_index] += current_intensity;
                } else {
                    break;
                }
            }
        }
        self.intensities = smooth_intensities;
        self
    }

    pub(crate) fn centroid(mut self, window: u32) -> Self {
        let local_maxima: Vec<bool> = find_sparse_local_maxima_mask(
            &self.tof_indices,
            &self.intensities,
            window,
        );
        self.tof_indices = filter_with_mask(&self.tof_indices, &local_maxima);
        self.intensities = filter_with_mask(&self.intensities, &local_maxima);
        self
    }

    pub(crate) fn finalize(&self, precursor: Precursor) -> Spectrum {
        let isolation_window = timsrust_core::IsolationWindow::new_from_center(
            Mz::from(self.isolation_mz),
            Mz::from(self.isolation_width),
            self.collision_energy,
        );
        Spectrum::new(
            self.intensities.iter().map(|x| *x as f64).collect(),
            precursor.index(),
            Some(precursor),
            self.tof_indices
                .iter()
                .map(|&x| TofIndex::try_from(x).unwrap())
                .collect(),
            isolation_window,
        )
        // Spectrum {
        //     intensities: self.intensities.iter().map(|x| *x as f64).collect(),
        //     index: precursor.index,
        //     precursor: Some(precursor),
        //     isolation_window,
        //     tof_indices: self
        //         .tof_indices
        //         .iter()
        //         .map(|&x| TofIndex::try_from(x).unwrap())
        //         .collect(),
        //     ..Default::default()
        // }
    }
}

pub(crate) enum RawSpectrumReader {
    Dda(DDARawSpectrumReader),
    Dia(DIARawSpectrumReader),
}

impl std::fmt::Debug for RawSpectrumReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawSpectrumReader {{ /* fields omitted */ }}")
    }
}

impl RawSpectrumReader {
    pub(crate) fn new<ImC: InvertibleConverter<ScanIndex, Im>>(
        tdf_sql_reader: &SqlReader,
        frame_reader: TdfFrameReader,
        acquisition_type: AcquisitionType,
        splitting_strategy: FrameWindowSplittingStrategy<ImC>,
    ) -> Result<Self, RawSpectrumReaderError> {
        let raw_spectrum_reader = match acquisition_type {
            AcquisitionType::DDAPASEF => Self::Dda(DDARawSpectrumReader::new(
                tdf_sql_reader,
                frame_reader,
            )?),
            AcquisitionType::DIAPASEF => Self::Dia(DIARawSpectrumReader::new(
                tdf_sql_reader,
                frame_reader,
                splitting_strategy,
            )?),
            acquisition_type => {
                return Err(RawSpectrumReaderError::UnsupportedAcquisition(
                    format!("{:?}", acquisition_type),
                ));
            },
        };
        Ok(raw_spectrum_reader)
    }

    pub(crate) fn get(
        &self,
        index: usize,
    ) -> Result<RawSpectrum, RawSpectrumReaderError> {
        match self {
            Self::Dda(reader) => reader.get(index),
            Self::Dia(reader) => reader.get(index),
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Dda(reader) => reader.len(),
            Self::Dia(reader) => reader.len(),
        }
    }
}

pub(crate) trait RawSpectrumReaderTrait: Sync + Send {
    fn get(&self, index: usize) -> Result<RawSpectrum, RawSpectrumReaderError>;
    fn len(&self) -> usize;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RawSpectrumReaderError {
    #[error("{0}")]
    DDARawSpectrumReaderError(#[from] DDARawSpectrumReaderError),
    #[error("{0}")]
    DIARawSpectrumReaderError(#[from] DIARawSpectrumReaderError),
    #[error("Invalid acquistion type for Raw spectrum reader: {0}")]
    UnsupportedAcquisition(String),
}
