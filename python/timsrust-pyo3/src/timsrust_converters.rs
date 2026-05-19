use timsrust::{
    ImConverter, MzConverter, RtConverter,
    core::{Converter, FrameIndex, Im, Mz, Rt, ScanIndex, TofIndex},
};

use pyo3::prelude::*;

#[derive(Clone)]
#[pyclass(name = "Frame2RtConverter")]
pub struct PyFrame2RtConverter {
    pub converter: RtConverter,
}

impl From<&RtConverter> for PyFrame2RtConverter {
    fn from(x: &RtConverter) -> Self {
        PyFrame2RtConverter {
            converter: x.clone(),
        }
    }
}

impl PyFrame2RtConverter {
    pub fn convert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let value = value.into() as u32;
        let value = FrameIndex::try_from(value).unwrap();
        f64::from(self.converter.convert(value))
    }

    pub fn invert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let x = value.into();
        let x = Rt::from(x as f32);
        f64::from(self.converter.convert(x))
    }
}

#[derive(Clone)]
#[pyclass(name = "ImConverter")]
pub struct PyScan2ImConverter {
    pub converter: ImConverter,
}

impl From<&ImConverter> for PyScan2ImConverter {
    fn from(x: &ImConverter) -> Self {
        PyScan2ImConverter {
            converter: x.clone(),
        }
    }
}

impl PyScan2ImConverter {
    pub fn convert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let value = value.into() as u32;
        let value = ScanIndex::try_from(value).unwrap();
        f64::from(self.converter.convert(value))
    }

    pub fn invert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let x = value.into();
        let x = Im::from(x as f32);
        f64::from(self.converter.convert(x))
    }
}

#[derive(Clone)]
#[pyclass(name = "MzConverter")]
pub struct PyTof2MzConverter {
    pub converter: MzConverter,
}

impl From<&MzConverter> for PyTof2MzConverter {
    fn from(x: &MzConverter) -> Self {
        PyTof2MzConverter {
            converter: x.clone(),
        }
    }
}

impl PyTof2MzConverter {
    pub fn convert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let value = value.into() as u32;
        let value = TofIndex::try_from(value).unwrap();
        f64::from(self.converter.convert(value))
    }

    pub fn invert<T: Into<f64> + Copy>(&self, value: T) -> f64 {
        let x = value.into();
        let x = Mz::from(x as f32);
        f64::from(self.converter.convert(x))
    }
}
