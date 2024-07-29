use super::Precursor;

/// An MS2 spectrum with centroided mz values and summed intensities.
#[derive(Debug, PartialEq, Default)]
pub struct Spectrum {
    pub mz_values: Vec<f64>,
    pub intensities: Vec<f64>,
    pub precursor: Option<Precursor>,
    pub index: usize,
    pub collision_energy: f64,
    pub isolation_mz: f64,
    pub isolation_width: f64,
}

impl Spectrum {
    pub fn get_top_n(&self, n: usize) -> Self {
        let top_n = if n == 0 { self.len() } else { n };
        let mut indexed: Vec<(f64, usize)> =
            self.intensities.iter().cloned().zip(0..).collect();
        // TODO
        indexed.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut top_indices: Vec<usize> = indexed
            .iter()
            .take(top_n)
            .map(|&(_, index)| index)
            .collect();
        top_indices.sort();
        Spectrum {
            mz_values: top_indices
                .iter()
                .map(|&index| self.mz_values[index])
                .collect(),
            intensities: top_indices
                .iter()
                .map(|&index| self.intensities[index])
                .collect(),
            precursor: self.precursor,
            index: self.index,
            collision_energy: self.collision_energy,
            isolation_mz: self.isolation_mz,
            isolation_width: self.isolation_width,
        }
    }

    pub fn len(&self) -> usize {
        self.mz_values.len()
    }
}
