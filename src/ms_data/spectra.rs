use super::Precursor;

/// An MS2 spectrum with centroided mz values and summed intensities.
#[derive(Debug, PartialEq, Default)]
pub struct Spectrum {
    pub mz_values: Vec<f64>,
    pub intensities: Vec<f64>,
    pub precursor: Precursor,
    pub index: usize,
    pub collision_energy: f64,
}
