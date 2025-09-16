use linreg::linear_regression;
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

use crate::readers::file_readers::sql_reader::calibrations::MzCalibration;

/// A converter from TOF -> m/z.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Tof2MzConverter {
    tof_intercept: f64,
    tof_slope: f64,
}

/// A converter from TOF -> m/z using the calibration parameters from the TDF files.
/// This is more complex but also more accurate.
///
/// In contrast with the other converter, this one needs to be instantiated
/// since it requires knowing some parameters that are specific to the frame
/// the converter is used for. (and uses more calibration information
/// stored in the TDF file).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Tof2MzConverter2 {
    c0: f64,
    c1: f64,
    digitizer_timebase: f64,
    delay: f64,
}

impl Tof2MzConverter2 {
    fn convert_f64(self, idx: f64) -> f64 {
        let time_of_flight = (idx * self.digitizer_timebase) + self.delay;
        let inner = time_of_flight - self.c0;

        (self.c1 * (inner.powi(2))) / 1e12
    }

    fn invert_f64(self, mz: f64) -> f64 {
        let time_of_flight = ((mz * 1e12) / self.c1).sqrt() + self.c0;

        (time_of_flight - self.delay) / self.digitizer_timebase
    }

    pub fn try_from_calibration(
        calibration: &MzCalibration,
        real_t1: f64,
        _real_t2: f64,
    ) -> Option<Self> {
        let MzCalibration {
            id: _,
            model_type,
            digitizer_timebase,
            digitizer_delay,
            t1,
            t2: _,
            dc1,
            dc2: _,
            c0,
            c1,
            c2: _,
            c3: _,
            c4: _,
        } = calibration;

        assert_eq!(*model_type, 1); // We only support model type 1 for now ... I do not even know
                                    // if more exist or whether this should be a recoverable error.

        let (c0, c1) = match (c0, c1) {
            (Some(c0), Some(c1)) => (*c0, *c1),
            (_, _) => return None,
        };

        // We can simplify this expression if dc2 is 0
        // cf = dc1 * (self.T1_reference - real_t1) + dc2 * (self.T2_reference - real_t2)
        let cf = dc1 * (t1 - real_t1);
        // Division is pretty expensive ... we can do this once ...
        // Although this is done only once per converter so its not thaaaat bad.
        let cf = 1.0 + (cf / 1.0e6);
        let c1_corrected = c1 * cf;
        Some(Self {
            c0,
            c1: c1_corrected,
            digitizer_timebase: *digitizer_timebase,
            delay: *digitizer_delay,
        })
    }
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
impl super::ConvertableDomain for Tof2MzConverter2 {
    fn convert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        self.convert_f64(value.into())
    }
    fn invert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        self.invert_f64(value.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tof2mz_converter2() {
        // These are real cal values from a TDF file
        // 1	1	0.125	25741	20.9410989491122	24.8706161298104	20	0	286.065160463331	154317.348188993	0.0	0.0	0.0
        let calibration = MzCalibration {
            id: 1,
            model_type: 1,
            digitizer_timebase: 0.125,
            digitizer_delay: 25741.0,
            t1: 20.9410989491122,
            t2: 24.8706161298104,
            dc1: 20.0,
            dc2: 0.0,
            c0: Some(286.065160463331),
            c1: Some(154317.348188993),
            c2: None,
            c3: None,
            c4: None,
        };
        // These are the two extreme values from a specific file
        // 20.9455139021767	24.7566839615201
        // 20.9485620654682	24.6520435267837
        //
        let real_t1 = 20.9455139021767;
        let real_t2 = 24.7566839615201;

        let converter2 = Tof2MzConverter2::try_from_calibration(
            &calibration,
            real_t1,
            real_t2,
        )
        .unwrap();
        // Now we can convert some values
        // Let's convert the first and last tof index from the file
        // 0 -> 636029 # From the global metadata table
        // Wich should loosely be ... 99.990834 - 1700.000000
        let mz_0 = converter2.convert_f64(0.0);
        let mz_636029 = converter2.convert_f64(636029.0);

        const TOL: f64 = 1e-3;
        assert!((mz_0 - 99.990834).abs() < TOL, "mz_0: {}", mz_0);
        assert!(
            (mz_636029 - 1700.005).abs() < TOL,
            "mz_636029: {}",
            mz_636029
        );

        // Test inversion
        let tof_0 = converter2.invert_f64(mz_0);
        let tof_636029 = converter2.invert_f64(mz_636029);

        assert!((tof_0 - 0.0).abs() < TOL, "tof_0: {}", tof_0);
        assert!(
            (tof_636029 - 636029.0).abs() < TOL,
            "tof_636029: {}",
            tof_636029
        );
    }
}
