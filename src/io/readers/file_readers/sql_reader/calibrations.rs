use super::{ParseDefault, ReadableSqlTable};

pub struct MzCalibration {
    pub(crate) id: u8,
    pub(crate) model_type: u8,
    pub(crate) digitizer_timebase: f64,
    pub(crate) digitizer_delay: f64,
    pub(crate) t1: f64,
    pub(crate) t2: f64,
    pub(crate) dc1: f64,
    pub(crate) dc2: f64,
    pub(crate) c0: Option<f64>,
    pub(crate) c1: Option<f64>,
    pub(crate) c2: Option<f64>,
    pub(crate) c3: Option<f64>,
    pub(crate) c4: Option<f64>,
}

impl ReadableSqlTable for MzCalibration {
    fn get_sql_query() -> String {
        "SELECT Id, ModelType, DigitizerTimebase, DigitizerDelay, T1, T2, dC1, dC2, C0 , C1, C2, C3, C4 from MzCalibration".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        Self {
            id: row.parse_default(0),
            model_type: row.parse_default(1),
            digitizer_timebase: row.parse_default(2),
            digitizer_delay: row.parse_default(3),
            t1: row.parse_default(4),
            t2: row.parse_default(5),
            dc1: row.parse_default(6),
            dc2: row.parse_default(7),
            c0: row.parse_default(8),
            c1: row.parse_default(9),
            c2: row.parse_default(10),
            c3: row.parse_default(11),
            c4: row.parse_default(12),
        }
    }
}

pub struct TimsCalibration {
    pub(crate) id: u8,
    model_type: u8,
    c0: Option<f64>,
    c1: Option<f64>,
    c2: Option<f64>,
    c3: Option<f64>,
    c4: Option<f64>,
    c5: Option<f64>,
    c6: Option<f64>,
    c7: Option<f64>,
    c8: Option<f64>,
    c9: Option<f64>,
}

impl TimsCalibration {
    pub(crate) fn convert_im(&self, scan_no: f64) -> f64 {
        return self
            .convert_im_iter(std::iter::once(scan_no))
            .next()
            .unwrap();
    }

    pub(crate) fn convert_im_iter<'a>(
        &'a self,
        scan_no_iter: impl Iterator<Item = f64> + 'a,
    ) -> impl Iterator<Item = f64> + 'a {
        let conv = self.get_conversion_function();
        scan_no_iter.map(conv)
    }

    pub fn get_conversion_function(&self) -> impl Fn(f64) -> f64 {
        // Inspired from:
        // https://github.com/Roestlab/dia-pasef/src/diapysef/sandbox/trystuff.py
        // Mobility[1/k0] = 1/(c6+c7/(c2+((c3-c2)/c1)*(scanno-c4-c0)))
        // Which seems to be under MIT
        //
        //
        let TimsCalibration {
            id: _,
            model_type,
            c0,
            c1,
            c2,
            c3,
            c4,
            c5: _,
            c6,
            c7,
            c8: _,
            c9: _,
        } = self;

        let (c0, c1, c2, c3, c4, c6, c7) = match (
            model_type, c0, c1, c2, c3, c4, c6, c7,
        ) {
            (
                2,
                Some(c0),
                Some(c1),
                Some(c2),
                Some(c3),
                Some(c4),
                Some(c6),
                Some(c7),
            ) => (*c0, *c1, *c2, *c3, *c4, *c6, *c7),
            (2, _, _, _, _, _, _, _) => {
                panic!("Invalid TimsCalibration missing coefficients for model_type 2");
            },
            (model_type, _, _, _, _, _, _, _) => {
                panic!("Invalid TimsCalibration with unsupported model_type {model_type}");
            },
        };

        // Old result ...
        // move |scan_no| {
        //     1.0 / (c6 + c7 / (c2 + ((c3 - c2) / c1) * (scan_no - c4 - c0)))
        // }

        // Pre-calculate constants (same as before)
        let slope = (c3 - c2) / c1;
        let offset = c2 - slope * (c4 + c0);
        move |scan_no| 1.0 / (c6 + c7 / (offset + slope * scan_no))
    }
}

impl ReadableSqlTable for TimsCalibration {
    fn get_sql_query() -> String {
        "SELECT Id, ModelType, C0 , C1, C2, C3, C4, C5, C6, C7, C8, C9 FROM TimsCalibration".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        Self {
            id: row.parse_default(0),
            model_type: row.parse_default(1),
            c0: row.parse_default(2),
            c1: row.parse_default(3),
            c2: row.parse_default(4),
            c3: row.parse_default(5),
            c4: row.parse_default(6),
            c5: row.parse_default(7),
            c6: row.parse_default(8),
            c7: row.parse_default(9),
            c8: row.parse_default(10),
            c9: row.parse_default(11),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_convert_im() {
        // From an ultra 2 calibration
        // Id ModelType C0 C1  C2               C3               C4               C5 C6 ...
        // 1  2         1  708 241.751905250524 99.2437539638487 33.9622641509434 1  0.0071422641733084 164.998795925213 16.3705403907576 2553.11607142569
        //
        // I would assume the pressure would be used somewhere 2.46155376641182 is an example value
        // for it in the frame table ...
        //
        // Notes:
        // - C1 and C2 seem to be the min-max scan indices.
        // - C9 almost seems like an m/z ... but the dll only takes frame id and scan
        //   numbers as input.
        //
        // let slope = (c3 - c2) / c1;
        // let offset = c2 - slope * (c4 + c0);
        // move |scan_no| 1.0 / (c6 + c7 / (offset + slope * scan_no))

        let calib = TimsCalibration {
            id: 1,
            model_type: 2,
            c0: Some(1.),
            c1: Some(708.),
            c2: Some(241.751905250524),
            c3: Some(99.2437539638487),
            c4: Some(33.9622641509434),
            c5: Some(1.0),
            c6: Some(0.0071422641733084),
            c7: Some(164.998795925213),
            c8: Some(16.3705403907576),
            c9: Some(2553.11607142569),
        };
        const TOL: f64 = 5e-2;
        let im = calib.convert_im(1.0);
        // 1.450000 is the set max IM
        assert!((im - 1.45).abs() < TOL, "im: {}", im);
        let im = calib.convert_im(708.0);
        // 0.640000 is the set min IM
        assert!((im - 0.64).abs() < TOL, "im: {}", im);
    }
}
