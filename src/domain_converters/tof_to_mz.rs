use linreg::linear_regression;
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

/// A converter from TOF -> m/z.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Tof2MzConverter {
    tof_intercept: f64,
    tof_slope: f64,
}

impl Tof2MzConverter {
    pub fn from_boundaries(
        mz_min: f64,
        mz_max: f64,
        tof_max_index: u32,
    ) -> Self {
        let tof_intercept: f64 = mz_min.sqrt();
        let tof_slope: f64 =
            (mz_max.sqrt() - tof_intercept) / tof_max_index as f64;
        Self {
            tof_intercept,
            tof_slope,
        }
    }

    pub fn regress_from_pairs(data: &Vec<(f64, u32)>) -> Self {
        let x: Vec<u32> = data.iter().map(|(_, x_val)| *x_val).collect();
        let y: Vec<f64> =
            data.iter().map(|(y_val, _)| (*y_val).sqrt()).collect();
        let (tof_slope, tof_intercept) = linear_regression(&x, &y).unwrap();
        Self {
            tof_intercept,
            tof_slope,
        }
    }
}

impl super::ConvertableDomain for Tof2MzConverter {
    fn convert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let tof_index: f64 = value.into();
        (self.tof_intercept + self.tof_slope * tof_index).powi(2)
    }
    fn invert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let mz_value: f64 = value.into();
        (mz_value.sqrt() - self.tof_intercept) / self.tof_slope
    }
}
