use crate::{
    converters::{ConvertableIndex, Tof2MzConverter},
    precursors::QuadrupoleEvent,
    vec_utils::{filter_with_mask, find_sparse_local_maxima_mask},
    Precursor,
};

pub struct RawSpectrumProcessor {
    pub raw_spectrum: RawSpectrum,
}

impl RawSpectrumProcessor {
    pub fn smooth(mut self, window: u32) -> Self {
        let mut smooth_intensities: Vec<u64> =
            self.raw_spectrum.intensities.clone();
        for (current_index, current_tof) in
            self.raw_spectrum.tof_indices.iter().enumerate()
        {
            let current_intensity: u64 =
                self.raw_spectrum.intensities[current_index];
            for (_next_index, next_tof) in self.raw_spectrum.tof_indices
                [current_index + 1..]
                .iter()
                .enumerate()
            {
                let next_index: usize = _next_index + current_index + 1;
                let next_intensity: u64 =
                    self.raw_spectrum.intensities[next_index];
                if (next_tof - current_tof) <= window {
                    smooth_intensities[current_index] += next_intensity;
                    smooth_intensities[next_index] += current_intensity;
                } else {
                    break;
                }
            }
        }
        self.raw_spectrum.intensities = smooth_intensities;
        self.raw_spectrum.processed_state =
            RawProcessedSpectrumState::SmoothedProfile;
        self
    }

    pub fn centroid(mut self, window: u32) -> Self {
        let local_maxima: Vec<bool> = self.find_local_maxima(window);
        self.raw_spectrum.tof_indices =
            filter_with_mask(&self.raw_spectrum.tof_indices, &local_maxima);
        self.raw_spectrum.intensities =
            filter_with_mask(&self.raw_spectrum.intensities, &local_maxima);
        self.raw_spectrum.processed_state =
            RawProcessedSpectrumState::Centroided;
        self
    }

    fn find_local_maxima(&self, window: u32) -> Vec<bool> {
        find_sparse_local_maxima_mask(
            &self.raw_spectrum.tof_indices,
            &self.raw_spectrum.intensities,
            window,
        )
    }

    pub fn finalize(
        &self,
        precursor: Precursor,
        mz_reader: &Tof2MzConverter,
    ) -> Spectrum {
        let index = self.raw_spectrum.index;
        let spectrum: Spectrum = Spectrum {
            mz_values: self
                .raw_spectrum
                .tof_indices
                .iter()
                .map(|&x| mz_reader.convert(x))
                .collect(),
            intensities: self
                .raw_spectrum
                .intensities
                .iter()
                .map(|x| *x as f64)
                .collect(),
            precursor: QuadrupoleEvent::Precursor(precursor),
            index: index,
        };
        spectrum
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RawProcessedSpectrumState {
    Profile,
    SmoothedProfile,
    Centroided,
    Unprocessed,
}

impl Default for RawProcessedSpectrumState {
    fn default() -> Self {
        Self::Unprocessed
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct RawSpectrum {
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u64>,
    pub processed_state: RawProcessedSpectrumState,
    pub index: usize,
}

/// An MS2 spectrum with centroided mz values and summed intensities.
#[derive(Debug, PartialEq, Default)]
pub struct Spectrum {
    pub mz_values: Vec<f64>,
    pub intensities: Vec<f64>,
    pub precursor: QuadrupoleEvent,
    pub index: usize,
}
