/// A converter from Frame -> retention time.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Frame2RtConverter {
    rt_values: Vec<f64>,
}

impl Frame2RtConverter {
    pub fn from_values(rt_values: Vec<f64>) -> Self {
        Self { rt_values }
    }
}

impl super::ConvertableDomain for Frame2RtConverter {
    fn convert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let lower_value: f64 = self.rt_values[value.into().floor() as usize];
        let upper_value: f64 = self.rt_values[value.into().ceil() as usize];
        (lower_value + upper_value) / 2.
    }
    fn invert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let rt_value = value.into();
        match self.rt_values.binary_search_by(|probe| {
            probe.partial_cmp(&rt_value).expect("Cannot handle NaNs")
        }) {
            Ok(index) => index as f64,
            Err(index) => match index {
                _ if (index > 0) && (index < self.rt_values.len()) => {
                    let start = self.rt_values[index - 1];
                    let end = self.rt_values[index];
                    index as f64 + (rt_value - start) / (end - start)
                },
                _ => index as f64,
            },
        }
    }
}
