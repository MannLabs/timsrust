use core::fmt;

use crate::{
    domain_converters::{ConvertableDomain, Tof2MzConverter},
    io::readers::{file_readers::sql_reader::SqlReader, FrameReader},
    ms_data::{AcquisitionType, Precursor, Spectrum},
    utils::vec_utils::{filter_with_mask, find_sparse_local_maxima_mask},
};

use super::{dda::DDARawSpectrumReader, dia::DIARawSpectrumReader};

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
    pub fn smooth(mut self, window: u32) -> Self {
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

    pub fn centroid(mut self, window: u32) -> Self {
        let local_maxima: Vec<bool> = find_sparse_local_maxima_mask(
            &self.tof_indices,
            &self.intensities,
            window,
        );
        self.tof_indices = filter_with_mask(&self.tof_indices, &local_maxima);
        self.intensities = filter_with_mask(&self.intensities, &local_maxima);
        self
    }

    pub fn finalize(
        &self,
        precursor: Precursor,
        mz_reader: &Tof2MzConverter,
    ) -> Spectrum {
        let index = self.index;
        let spectrum: Spectrum = Spectrum {
            mz_values: self
                .tof_indices
                .iter()
                .map(|&x| mz_reader.convert(x))
                .collect(),
            intensities: self.intensities.iter().map(|x| *x as f64).collect(),
            precursor: Some(precursor),
            index: index,
            collision_energy: self.collision_energy,
            isolation_mz: self.isolation_mz,
            isolation_width: self.isolation_width,
        };
        spectrum
    }
}

pub struct RawSpectrumReader {
    raw_spectrum_reader: Box<dyn RawSpectrumReaderTrait>,
}

impl fmt::Debug for RawSpectrumReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawSpectrumReader {{ /* fields omitted */ }}")
    }
}

impl RawSpectrumReader {
    pub fn new(
        tdf_sql_reader: &SqlReader,
        frame_reader: FrameReader,
        acquisition_type: AcquisitionType,
    ) -> Self {
        let raw_spectrum_reader: Box<dyn RawSpectrumReaderTrait> =
            match acquisition_type {
                AcquisitionType::DDAPASEF => Box::new(
                    DDARawSpectrumReader::new(tdf_sql_reader, frame_reader),
                ),
                AcquisitionType::DIAPASEF => Box::new(
                    DIARawSpectrumReader::new(tdf_sql_reader, frame_reader),
                ),
                _ => panic!(),
            };
        Self {
            raw_spectrum_reader,
        }
    }

    pub fn get(&self, index: usize) -> RawSpectrum {
        self.raw_spectrum_reader.get(index)
    }

    pub fn len(&self) -> usize {
        self.raw_spectrum_reader.len()
    }
}

pub trait RawSpectrumReaderTrait: Sync {
    fn get(&self, index: usize) -> RawSpectrum;
    fn len(&self) -> usize;
}
