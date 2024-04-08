use crate::{
    converters::{ConvertableDomain, Tof2MzConverter},
    spectra::RawSpectrum,
    Precursor,
};

pub struct Tof2MzCalibrator;

impl Tof2MzCalibrator {
    pub fn find_unfragmented_precursors(
        spectra: &Vec<RawSpectrum>,
        mz_reader: &Tof2MzConverter,
        precursors: &Vec<Precursor>,
        tolerance: f64,
    ) -> Vec<(f64, u32)> {
        let mut hits: Vec<(f64, u32)> = vec![];
        for (index, spectrum) in spectra.iter().enumerate() {
            let precursor_mz: f64 = precursors[index].mz;
            for &tof_index in spectrum.tof_indices.iter() {
                let mz = mz_reader.convert(tof_index);
                if (mz - precursor_mz).abs() < tolerance {
                    let hit = (precursor_mz, tof_index);
                    hits.push(hit);
                }
            }
        }
        hits
    }
}
