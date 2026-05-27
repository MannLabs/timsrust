use timsrust_utils::vec::get_top_n;

use super::Precursor;
use crate::{IsolationWindow, Mz, TofIndex, coordinates::Converter};

/// An MS2 spectrum with centroided mz values and summed intensities.
#[derive(Debug, PartialEq, Default, Clone)]
pub struct Spectrum<C = TofIndex> {
    intensities: Vec<f64>,
    precursor: Option<Precursor>,
    index: usize,
    coordinates: Vec<C>,
    isolation_window: IsolationWindow,
}

impl<C> Spectrum<C> {
    pub fn new(
        intensities: Vec<f64>,
        index: usize,
        precursor: Option<Precursor>,
        coordinates: Vec<C>,
        isolation_window: IsolationWindow,
    ) -> Self {
        assert!(
            intensities.len() == coordinates.len(),
            "Intensities and coordinates must have the same length"
        );
        Spectrum {
            intensities,
            precursor,
            index,
            coordinates,
            isolation_window,
        }
    }

    pub fn intensities(&self) -> &Vec<f64> {
        &self.intensities
    }

    pub fn precursor(&self) -> &Option<Precursor> {
        &self.precursor
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn isolation_window(&self) -> &IsolationWindow {
        &self.isolation_window
    }

    pub fn len(&self) -> usize {
        self.intensities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn coordinates(&self) -> &Vec<C> {
        &self.coordinates
    }

    pub fn convert_to<X>(self, converter: impl Converter<C, X>) -> Spectrum<X>
    where
        C: Copy,
    {
        Spectrum {
            intensities: self.intensities,
            precursor: self.precursor,
            index: self.index,
            coordinates: converter.batch_convert(&self.coordinates),
            isolation_window: self.isolation_window,
        }
    }

    pub fn get_top_n(&self, n: usize) -> Self
    where
        C: Clone,
    {
        let top_indices = get_top_n(&self.intensities, n);
        Self {
            intensities: top_indices
                .iter()
                .map(|&index| self.intensities[index])
                .collect(),
            precursor: self.precursor.clone(),
            index: self.index,
            coordinates: top_indices
                .iter()
                .map(|&index| self.coordinates[index].clone())
                .collect(),
            isolation_window: self.isolation_window.clone(),
        }
    }
}

impl Spectrum<TofIndex> {
    pub fn tof_indices(&self) -> &Vec<TofIndex> {
        &self.coordinates
    }

    pub fn mz_values(
        &self,
        converter: impl Converter<TofIndex, Mz>,
    ) -> Vec<Mz> {
        converter.batch_convert(&self.coordinates)
    }

    pub fn to_mz_spectrum(
        self,
        converter: impl Converter<TofIndex, Mz>,
    ) -> Spectrum<Mz> {
        self.convert_to(converter)
    }
}

impl Spectrum<Mz> {
    pub fn mz_values(&self) -> &Vec<Mz> {
        &self.coordinates
    }

    pub fn to_tof_spectrum(
        self,
        converter: impl Converter<Mz, TofIndex>,
    ) -> Spectrum<TofIndex> {
        self.convert_to(converter)
    }
}
