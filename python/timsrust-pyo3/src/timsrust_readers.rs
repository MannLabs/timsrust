use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;
use rayon::iter::ParallelIterator;
use timsrust::SpectrumReader;
use timsrust::core::AcquisitionType;
use timsrust::core::MSLevel;
use timsrust::tdf::TdfFrameReader;
use timsrust::tdf::{
    FrameWindowSplittingConfiguration, QuadWindowExpansionStrategy,
    SpectrumProcessingParams, SpectrumReaderConfig,
};

use crate::timsrust_structs::PyFrame;
use crate::timsrust_structs::PySpectrum;
use std::sync::Arc;

#[pyclass(name = "FrameReader")]
pub struct PyFrameReader {
    pub reader: TdfFrameReader,
    pub i: usize,
}

#[pymethods]
impl PyFrameReader {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        Ok(PyFrameReader {
            reader: match TdfFrameReader::new(path) {
                Ok(x) => x,
                Err(_) => {
                    return Err(PyIOError::new_err("Could not open file"));
                },
            },
            i: 0,
        })
    }

    pub fn read_frame(&self, index: usize) -> PyResult<PyFrame> {
        match self.reader.get_frame(index) {
            Ok(x) => Ok(PyFrame::from(x)),
            Err(_) => {
                Err(PyIOError::new_err("Could not read frame, Corrupt frame"))
            },
        }
    }

    pub fn read_all_frames(&self) -> PyResult<Vec<PyFrame>> {
        self.reader
            .parallel_filter(|_| true)
            .map(|x| match x {
                Ok(x) => Ok(PyFrame::from(x)),
                Err(_) => Err(PyIOError::new_err(
                    "Could not read frame, Corrupt frame",
                )),
            })
            .collect()
    }

    pub fn read_dia_frames(&self) -> PyResult<Vec<PyFrame>> {
        self.reader
            .parallel_filter(|x| {
                (x.info().acquisition_type() == AcquisitionType::DIAPASEF)
                    && (x.info().ms_level() == MSLevel::MS2)
            })
            .map(|x| match x {
                Ok(x) => Ok(PyFrame::from(x)),
                Err(_) => Err(PyIOError::new_err(
                    "Could not read frame, Corrupt frame",
                )),
            })
            .collect()
    }

    pub fn read_ms1_frames(&self) -> PyResult<Vec<PyFrame>> {
        self.reader
            .parallel_filter(|x| x.info().ms_level() == MSLevel::MS1)
            .map(|x| match x {
                Ok(x) => Ok(PyFrame::from(x)),
                Err(_) => Err(PyIOError::new_err(
                    "Could not read frame, Corrupt frame",
                )),
            })
            .collect()
    }

    pub fn __len__(&self) -> usize {
        self.reader.len()
    }

    pub fn __iter__(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.i = 0;
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PyFrame>> {
        if slf.i < slf.reader.len() {
            let x = slf.reader.get_frame(slf.i);
            slf.i += 1;
            match x {
                Ok(x) => Ok(Some(PyFrame::from(x))),
                Err(_) => Err(PyIOError::new_err(
                    "Could not read spectrum, Corrupt spectrum",
                )),
            }
        } else {
            Ok(None)
        }
    }
}

#[pyclass(name = "SpectrumReader")]
pub struct PySpectrumReader {
    pub reader: Arc<SpectrumReader>,
    i: usize, // Using here so I can implement __iter__  and __next__
}

#[pymethods]
impl PySpectrumReader {
    #[new]
    pub fn new(path: &str) -> PyResult<Self> {
        match SpectrumReader::build().with_path(path).finalize() {
            Ok(x) => Ok(PySpectrumReader {
                reader: Arc::new(x),
                i: 0,
            }),
            Err(e) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    #[staticmethod]
    fn new_with_span_step(
        _py: Python<'_>,
        path: &str,
        mobility_span: f64,
        mobility_step: f64,
    ) -> PyResult<Self> {
        let params = SpectrumReaderConfig {
            frame_splitting_params:
                FrameWindowSplittingConfiguration::Quadrupole(
                    QuadWindowExpansionStrategy::UniformMobility(
                        (mobility_span, mobility_step),
                        None,
                    ),
                ),
            spectrum_processing_params: SpectrumProcessingParams::default(),
        };

        let builder = SpectrumReader::build()
            .with_path(path)
            .with_config(params)
            .finalize();
        match builder {
            Ok(x) => Ok(PySpectrumReader {
                reader: Arc::new(x),
                i: 0,
            }),
            Err(e) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    pub fn __len__(&self) -> usize {
        self.reader.len()
    }

    pub fn get(&self, index: usize) -> PyResult<PySpectrum> {
        match self.reader.get(index) {
            Ok(x) => Ok(PySpectrum::from(x)),
            Err(_) => Err(PyIOError::new_err(
                "Could not read spectrum, Corrupt spectrum",
            )),
        }
    }

    pub fn __iter__(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.i = 0;
        slf
    }

    pub fn __next__(
        mut slf: PyRefMut<'_, Self>,
    ) -> PyResult<Option<PySpectrum>> {
        if slf.i < slf.reader.len() {
            let x = slf.reader.get(slf.i);
            slf.i += 1;
            match x {
                Ok(x) => Ok(Some(PySpectrum::from(x))),
                Err(_) => Err(PyIOError::new_err(
                    "Could not read spectrum, Corrupt spectrum",
                )),
            }
        } else {
            Ok(None)
        }
    }
}
