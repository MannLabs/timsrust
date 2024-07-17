/// A converter from Scan -> (inversed) ion mobility.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Scan2ImConverter {
    scan_intercept: f64,
    scan_slope: f64,
}

impl Scan2ImConverter {
    pub fn from_boundaries(
        im_min: f64,
        im_max: f64,
        scan_max_index: u32,
    ) -> Self {
        let scan_intercept: f64 = im_max;
        let scan_slope: f64 = (im_min - scan_intercept) / scan_max_index as f64;
        Self {
            scan_intercept,
            scan_slope,
        }
    }
}

impl super::ConvertableDomain for Scan2ImConverter {
    fn convert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let scan_index_f64: f64 = value.into();
        self.scan_intercept + self.scan_slope * scan_index_f64
    }
}
