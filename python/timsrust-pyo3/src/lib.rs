pub mod timsrust_converters;
pub mod timsrust_enums;
pub mod timsrust_readers;
pub mod timsrust_structs;

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;
use timsrust::tdf::TdfFrameReader;

use crate::timsrust_enums::{PyAcquisitionType, PyMSLevel};
use crate::timsrust_readers::{PyFrameReader, PySpectrumReader};
use crate::timsrust_structs::{
    PyFrame, PyMetadata, PyPrecursor, PyQuadrupoleSettings, PySpectrum,
};

#[pyfunction]
fn read_all_frames(path: String) -> PyResult<Vec<PyFrame>> {
    let reader = TdfFrameReader::new(&path).unwrap();
    let tims_reader = PyFrameReader { reader, i: 0 };
    tims_reader.read_all_frames()
}

#[pyfunction]
fn read_all_spectra(path: String) -> PyResult<Vec<PySpectrum>> {
    let reader = timsrust::SpectrumReader::build()
        .with_path(&path)
        .finalize();

    if reader.is_err() {
        return Err(PyIOError::new_err(reader.err().unwrap().to_string()));
    }
    let reader = reader.unwrap();
    reader
        .get_all()
        .into_iter()
        .map(|x| match x {
            Ok(x) => Ok(PySpectrum::from(x)),
            Err(e) => Err(PyIOError::new_err(e.to_string())),
        })
        .collect()
}

#[pymodule]
fn tims_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(read_all_frames, m)?)?;
    m.add_function(wrap_pyfunction!(read_all_spectra, m)?)?;
    m.add_class::<PyFrame>()?;
    m.add_class::<PyFrameReader>()?;
    m.add_class::<PySpectrumReader>()?;
    m.add_class::<PyMetadata>()?;
    m.add_class::<PyPrecursor>()?;
    m.add_class::<PyQuadrupoleSettings>()?;
    m.add_class::<PySpectrum>()?;
    m.add_class::<PyAcquisitionType>()?;
    m.add_class::<PyMSLevel>()?;
    Ok(())
}
