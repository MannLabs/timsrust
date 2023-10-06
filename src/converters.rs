use linreg::linear_regression;

pub trait ConvertableIndex {
    fn convert<T: Into<f64> + Copy>(&self, index: T) -> f64;
}

#[derive(Debug, Copy, Clone)]
pub struct Tof2MzConverter {
    tof_intercept: f64,
    tof_slope: f64,
}

impl Tof2MzConverter {
    pub fn new(mz_min: f64, mz_max: f64, tof_max_index: u32) -> Self {
        let tof_intercept: f64 = mz_min.sqrt();
        let tof_slope: f64 =
            (mz_max.sqrt() - tof_intercept) / tof_max_index as f64;
        Self {
            tof_intercept,
            tof_slope,
        }
    }

    pub fn from_unfragmented_precursors(data: &Vec<(f64, u32)>) -> Self {
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

impl ConvertableIndex for Tof2MzConverter {
    fn convert<T: Into<f64> + Copy>(&self, index: T) -> f64 {
        let tof_index_f64: f64 = index.into();
        (self.tof_intercept + self.tof_slope * tof_index_f64).powi(2)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Scan2ImConverter {
    scan_intercept: f64,
    scan_slope: f64,
}

impl Scan2ImConverter {
    pub fn new(im_min: f64, im_max: f64, scan_max_index: u32) -> Self {
        let scan_intercept: f64 = im_max;
        let scan_slope: f64 = (im_min - scan_intercept) / scan_max_index as f64;
        Self {
            scan_intercept,
            scan_slope,
        }
    }
}

impl ConvertableIndex for Scan2ImConverter {
    fn convert<T: Into<f64> + Copy>(&self, index: T) -> f64 {
        let scan_index_f64: f64 = index.into();
        self.scan_intercept + self.scan_slope * scan_index_f64
    }
}

#[derive(Debug, Clone)]
pub struct Frame2RtConverter {
    rt_values: Vec<f64>,
}

impl Frame2RtConverter {
    pub fn new(rt_values: Vec<f64>) -> Self {
        Self { rt_values }
    }
}

impl ConvertableIndex for Frame2RtConverter {
    fn convert<T: Into<f64> + Copy>(&self, index: T) -> f64 {
        let lower_value: f64 = self.rt_values[index.into().floor() as usize];
        let upper_value: f64 = self.rt_values[index.into().ceil() as usize];
        (lower_value + upper_value) / 2.
    }
}
