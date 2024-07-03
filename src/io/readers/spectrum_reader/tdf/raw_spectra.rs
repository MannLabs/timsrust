use crate::{
    domain_converters::{ConvertableDomain, Tof2MzConverter},
    ms_data::{Precursor, Spectrum},
    utils::vec_utils::{filter_with_mask, find_sparse_local_maxima_mask},
};

#[derive(Debug, PartialEq, Default, Clone)]
pub(crate) struct RawSpectrum {
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u64>,
    pub index: usize,
    pub collision_energy: f64,
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
            precursor: precursor,
            index: index,
            collision_energy: self.collision_energy,
        };
        spectrum
    }
}
