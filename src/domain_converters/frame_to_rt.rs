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
}
