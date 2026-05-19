use std::fmt::Display;

use pyo3::prelude::*;

use timsrust::core::{AcquisitionType, MSLevel};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[pyclass(eq, eq_int, name = "AcquisitionType")]
pub enum PyAcquisitionType {
    #[pyo3(name = "DDAPASEF")]
    DDAPASEF,
    #[pyo3(name = "DIAPASEF")]
    DIAPASEF,
    #[pyo3(name = "DiagonalDIAPASEF")]
    DiagonalDIAPASEF,
    #[pyo3(name = "Unknown")]
    Unknown,
}

impl Display for PyAcquisitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PyAcquisitionType::DDAPASEF => "DDAPASEF",
                PyAcquisitionType::DIAPASEF => "DIAPASEF",
                PyAcquisitionType::DiagonalDIAPASEF => "DiagonalDIAPASEF",
                PyAcquisitionType::Unknown => "Unknown",
            }
        )
    }
}

impl From<&AcquisitionType> for PyAcquisitionType {
    fn from(x: &AcquisitionType) -> Self {
        match x {
            AcquisitionType::DDAPASEF => PyAcquisitionType::DDAPASEF,
            AcquisitionType::DIAPASEF => PyAcquisitionType::DIAPASEF,
            AcquisitionType::DiagonalDIAPASEF => {
                PyAcquisitionType::DiagonalDIAPASEF
            },
            AcquisitionType::Unknown => PyAcquisitionType::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[pyclass(eq, eq_int, name = "MSLevel")]
pub enum PyMSLevel {
    #[pyo3(name = "MS1")]
    MS1,
    #[pyo3(name = "MS2")]
    MS2,
    #[pyo3(name = "Unknown")]
    Unknown,
}

impl From<&MSLevel> for PyMSLevel {
    fn from(x: &MSLevel) -> Self {
        match x {
            MSLevel::MS1 => PyMSLevel::MS1,
            MSLevel::MS2 => PyMSLevel::MS2,
            MSLevel::Unknown => PyMSLevel::Unknown,
        }
    }
}

impl Display for PyMSLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PyMSLevel::MS1 => "MS1",
                PyMSLevel::MS2 => "MS2",
                PyMSLevel::Unknown => "Unknown",
            }
        )
    }
}
