//! Direct, bounded access to native observations and their acquisition metadata.

use pyo3::{
    exceptions::{PyIOError, PyIndexError, PyValueError},
    prelude::*,
    types::{PyDict, PyList},
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use timsrust::{
    NativeNeighborhoodReader,
    centroid::neighborhood::{
        AcquisitionPolicy, Anchor, BoundaryPolicy, Config, ContextExtent,
        ImSizing, Neighborhood, Observation, ObservationDomain,
        RtConsistencyConfig, RtSelectionPolicy, View,
    },
    core::{FrameInfo, IsolationWindow, Precursor, QuadrupoleSettings},
};
use timsrust_parquet_spectra::neighborhoods::{AcquisitionRow, Reader, Writer};

const MAX_BATCH: usize = 1024;

struct AcquisitionMetadata {
    frames: BTreeMap<u64, Vec<AcquisitionRow>>,
}

impl AcquisitionMetadata {
    fn new(rows: Vec<AcquisitionRow>) -> Self {
        let mut frames = BTreeMap::<_, Vec<_>>::new();
        for row in rows {
            frames.entry(row.frame).or_default().push(row);
        }
        for rows in frames.values_mut() {
            rows.sort_by_key(|r| r.interval);
        }
        Self { frames }
    }

    fn for_frame(&self, frame: u64) -> &[AcquisitionRow] {
        self.frames.get(&frame).map(Vec::as_slice).unwrap_or(&[])
    }
}

fn io_error(error: impl std::fmt::Display) -> PyErr {
    PyIOError::new_err(error.to_string())
}

fn value_error(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}

// Convert directly to Python objects, keeping integer values out of f64.
fn python_value(py: Python<'_>, value: Value) -> PyResult<Py<PyAny>> {
    Ok(match value {
        Value::Null => py.None(),
        Value::Bool(v) => v.into_pyobject(py)?.to_owned().into_any().unbind(),
        Value::Number(v) => {
            if let Some(n) = v.as_u64() {
                n.into_pyobject(py)?.into_any().unbind()
            } else if let Some(n) = v.as_i64() {
                n.into_pyobject(py)?.into_any().unbind()
            } else {
                v.as_f64()
                    .ok_or_else(|| value_error("invalid numeric metadata"))?
                    .into_pyobject(py)?
                    .into_any()
                    .unbind()
            }
        },
        Value::String(v) => v.into_pyobject(py)?.into_any().unbind(),
        Value::Array(values) => {
            let list = PyList::empty(py);
            for value in values {
                list.append(python_value(py, value)?)?;
            }
            list.into_any().unbind()
        },
        Value::Object(values) => {
            let dict = PyDict::new(py);
            for (key, value) in values {
                dict.set_item(key, python_value(py, value)?)?;
            }
            dict.into_any().unbind()
        },
    })
}

fn metadata(py: Python<'_>, value: &impl Serialize) -> PyResult<Py<PyAny>> {
    python_value(py, serde_json::to_value(value).map_err(value_error)?)
}

/// Optional RT qualification; peak admission defaults to native-preserving.
#[pyclass(name = "RtConsistencyConfig", module = "tims_pyo3", frozen)]
#[derive(Clone)]
pub(crate) struct PyRtConsistencyConfig {
    inner: RtConsistencyConfig,
}

#[pymethods]
impl PyRtConsistencyConfig {
    #[new]
    #[pyo3(signature = (*, mz_ppm=10.0, minimum_support=3, require_both_sides=true, selection="native_preserving"))]
    fn new(
        mz_ppm: f64,
        minimum_support: usize,
        require_both_sides: bool,
        selection: &str,
    ) -> PyResult<Self> {
        let selection = match selection {
            "native_preserving" => RtSelectionPolicy::NativePreserving,
            "support_weighted" => RtSelectionPolicy::SupportWeighted,
            "focal_response" => RtSelectionPolicy::FocalResponse,
            _ => return Err(value_error("unknown RT selection policy")),
        };
        let inner = RtConsistencyConfig {
            mz_ppm,
            minimum_support,
            require_both_sides,
            selection,
        };
        inner.validate().map_err(value_error)?;
        Ok(Self { inner })
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        metadata(py, &self.inner)
    }
}

/// Validated keyword-only observation geometry and native measurement policy.
#[pyclass(name = "NativeNeighborhoodConfig", module = "tims_pyo3", frozen)]
#[derive(Clone)]
pub(crate) struct PyNativeNeighborhoodConfig {
    inner: Config,
}

#[pymethods]
impl PyNativeNeighborhoodConfig {
    #[new]
    #[pyo3(signature = (*, acquisition="single_interval", rt_context=(2, 2),
        im_context=(0, 0), im_scans=None, im_padding=None, peak_cap=300,
        min_ms2_ion_count=2, require_complete=false, rt_consistency=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        acquisition: &str,
        rt_context: (usize, usize),
        im_context: (usize, usize),
        im_scans: Option<(usize, usize, usize)>,
        im_padding: Option<(f64, f64)>,
        peak_cap: usize,
        min_ms2_ion_count: u32,
        require_complete: bool,
        rt_consistency: Option<PyRef<'_, PyRtConsistencyConfig>>,
    ) -> PyResult<Self> {
        if im_scans.is_some() && im_padding.is_some() {
            return Err(value_error(
                "im_scans and im_padding are mutually exclusive",
            ));
        }
        let acquisition = match acquisition {
            "single_interval" => AcquisitionPolicy::SingleInterval,
            "shared_transmission" => AcquisitionPolicy::SharedTransmission,
            "experimental_trajectory" => {
                AcquisitionPolicy::ExperimentalTrajectory
            },
            _ => return Err(value_error("unknown acquisition policy")),
        };
        let im = match im_scans {
            Some((left, right, stride)) => ImSizing::Scans {
                left,
                right,
                stride,
            },
            None => {
                let (padding_left, padding_right) =
                    im_padding.unwrap_or((0.0, 0.0));
                ImSizing::NativeFwhm {
                    padding_left,
                    padding_right,
                }
            },
        };
        let inner = Config {
            acquisition,
            rt_context: ContextExtent {
                before: rt_context.0,
                after: rt_context.1,
            },
            im_context: ContextExtent {
                before: im_context.0,
                after: im_context.1,
            },
            im,
            peak_cap,
            min_ms2_ion_count: f64::from(min_ms2_ion_count),
            boundary: if require_complete {
                BoundaryPolicy::RequireComplete
            } else {
                BoundaryPolicy::KeepMasked
            },
            rt_consistency: rt_consistency.map(|c| c.inner.clone()),
            ..Config::default()
        };
        inner.validate().map_err(value_error)?;
        Ok(Self { inner })
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        metadata(py, &self.inner)
    }
}

fn acquisition_rows(frames: &[FrameInfo]) -> Vec<AcquisitionRow> {
    frames
        .iter()
        .flat_map(|frame| {
            let q = frame.quadrupole_settings();
            (0..q.len()).map(move |interval| AcquisitionRow {
                frame: frame.index() as u64,
                interval: interval as u64,
                group: frame.window_group(),
                rt_s: frame.rt_in_seconds(),
                scan_start: q.scan_starts[interval] as u64,
                scan_end: q.scan_ends[interval] as u64,
                isolation_lower: f64::from(
                    q.isolation_windows[interval].lower(),
                ),
                isolation_upper: f64::from(
                    q.isolation_windows[interval].upper(),
                ),
                collision_energy: q.isolation_windows[interval]
                    .collision_energy(),
            })
        })
        .collect()
}

fn precursor(py: Python<'_>, p: &Precursor) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("mz", f64::from(p.mz()))?;
    d.set_item("rt_s", f64::from(p.rt()))?;
    d.set_item("inverse_mobility", f64::from(p.im()))?;
    d.set_item("scan", u32::from(p.scan_index()))?;
    d.set_item("frame", u32::from(p.frame_index()))?;
    d.set_item("charge", p.charge().map(i8::from))?;
    d.set_item("intensity", *p.intensity())?;
    d.set_item("index", p.index())?;
    Ok(d.into_any().unbind())
}

fn view(
    py: Python<'_>,
    v: &View,
    rows: &AcquisitionMetadata,
) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("frame", v.anchor.frame)?;
    d.set_item("interval", v.anchor.interval)?;
    d.set_item("scan", v.anchor.scan)?;
    d.set_item("rt_s", v.rt_s)?;
    d.set_item("inverse_mobility", v.inverse_mobility)?;
    d.set_item("requested_scan_start", v.requested_scan_start)?;
    d.set_item("requested_scan_end", v.requested_scan_end)?;
    d.set_item("scan_start", v.scan_start)?;
    d.set_item("scan_end", v.scan_end)?;
    d.set_item("fully_covered", v.fully_covered)?;
    d.set_item("spectrum_id", v.spectrum.index())?;
    let window = v.spectrum.isolation_window();
    let summary = PyDict::new(py);
    summary.set_item(
        "meaning",
        "conservative_selected_source_support_intersection",
    )?;
    summary.set_item("isolation_lower", f64::from(window.lower()))?;
    summary.set_item("isolation_upper", f64::from(window.upper()))?;
    summary.set_item("collision_energy", window.collision_energy())?;
    d.set_item("isolation_summary", summary)?;
    d.set_item(
        "precursor",
        v.spectrum
            .precursor()
            .as_ref()
            .map(|p| precursor(py, p))
            .transpose()?,
    )?;

    let frame_rows = rows.for_frame(v.anchor.frame as u64);
    d.set_item("acquisition", metadata(py, &frame_rows)?)?;
    if let Some(p) = v.spectrum.precursor() {
        let q = QuadrupoleSettings {
            index: 0,
            scan_starts: frame_rows
                .iter()
                .map(|r| r.scan_start as usize)
                .collect(),
            scan_ends: frame_rows.iter().map(|r| r.scan_end as usize).collect(),
            isolation_windows: frame_rows
                .iter()
                .map(|r| {
                    IsolationWindow::new_from_bounds(
                        r.isolation_lower.into(),
                        r.isolation_upper.into(),
                        r.collision_energy,
                    )
                })
                .collect(),
        };
        let domain = ObservationDomain::new(
            &q,
            v.requested_scan_start,
            v.requested_scan_end,
            f64::from(p.mz()),
        )
        .map_err(value_error)?;
        d.set_item("observation_domain", metadata(py, &domain)?)?;
    } else {
        d.set_item("observation_domain", py.None())?;
    }
    let n = v.spectrum.len();
    if v.peak_scan_tof.len() != n
        || v.peak_apex_responses.len() != n
        || v.peak_inverse_mobility.len() != n
        || v.rt_evidence.as_ref().is_some_and(|e| e.len() != n)
    {
        return Err(value_error("unaligned native peak metadata"));
    }
    let peaks = PyList::empty(py);
    for i in 0..n {
        let p = PyDict::new(py);
        p.set_item("scan", v.peak_scan_tof[i].0)?;
        p.set_item("tof", v.peak_scan_tof[i].1)?;
        p.set_item("mz", f64::from(v.spectrum.mz_values()[i]))?;
        p.set_item("inverse_mobility", v.peak_inverse_mobility[i])?;
        p.set_item("apex_response", v.peak_apex_responses[i])?;
        p.set_item(
            "rt_evidence",
            v.rt_evidence
                .as_ref()
                .map(|e| metadata(py, &e[i]))
                .transpose()?,
        )?;
        peaks.append(p)?;
    }
    d.set_item("rt_evidence_present", v.rt_evidence.is_some())?;
    d.set_item("peaks", peaks)?;
    Ok(d.into_any().unbind())
}

fn neighborhood(
    py: Python<'_>,
    n: &Neighborhood,
    rows: &AcquisitionMetadata,
    measured: Option<&Precursor>,
) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("focal", view(py, &n.focal, rows)?)?;
    d.set_item(
        "measured_precursor",
        measured.map(|p| precursor(py, p)).transpose()?,
    )?;
    let context = PyList::empty(py);
    for observation in &n.context {
        let o = PyDict::new(py);
        match observation {
            Observation::Available { offset, view: v } => {
                o.set_item("rt_offset", offset.rt)?;
                o.set_item("im_offset", offset.im)?;
                o.set_item("available", true)?;
                o.set_item("view", view(py, v, rows)?)?;
            },
            Observation::Unavailable {
                offset,
                frame,
                reason,
            } => {
                o.set_item("rt_offset", offset.rt)?;
                o.set_item("im_offset", offset.im)?;
                o.set_item("available", false)?;
                o.set_item("frame", *frame)?;
                o.set_item("reason", format!("{reason:?}"))?;
                o.set_item("view", py.None())?;
            },
        }
        context.append(o)?;
    }
    d.set_item("context", context)?;
    Ok(d.into_any().unbind())
}

fn queries(
    anchors: Vec<(usize, usize, usize)>,
    mzs: Option<Vec<f64>>,
) -> PyResult<Vec<(Anchor, Option<f64>)>> {
    if anchors.is_empty() || anchors.len() > MAX_BATCH {
        return Err(value_error("queries must contain 1..=1024 anchors"));
    }
    if mzs.as_ref().is_some_and(|m| m.len() != anchors.len()) {
        return Err(value_error("precursor_mzs must align with anchors"));
    }
    anchors
        .into_iter()
        .enumerate()
        .map(|(i, (frame, interval, scan))| {
            let mz = mzs.as_ref().map(|m| m[i]);
            validate_mz(mz)?;
            Ok((
                Anchor {
                    frame,
                    interval,
                    scan,
                },
                mz,
            ))
        })
        .collect()
}

fn validate_mz(mz: Option<f64>) -> PyResult<()> {
    if mz.is_some_and(|m| !m.is_finite() || m <= 0.0) {
        return Err(value_error("precursor_mz must be finite and positive"));
    }
    Ok(())
}

/// Explicit bounded observations from a raw calibrated diaPASEF acquisition.
#[pyclass(name = "NativeNeighborhoodReader", module = "tims_pyo3")]
pub(crate) struct PyNativeNeighborhoodReader {
    inner: NativeNeighborhoodReader,
    acquisition: AcquisitionMetadata,
}

impl PyNativeNeighborhoodReader {
    fn extract(
        &self,
        anchor: Anchor,
        mz: Option<f64>,
    ) -> PyResult<Neighborhood> {
        match mz {
            Some(mz) => self.inner.neighborhood_for_precursor(anchor, mz),
            None => self.inner.neighborhood_at(anchor),
        }
        .map_err(io_error)
    }
}

#[pymethods]
impl PyNativeNeighborhoodReader {
    #[new]
    #[pyo3(signature = (path, config=None))]
    fn new(
        py: Python<'_>,
        path: String,
        config: Option<PyRef<'_, PyNativeNeighborhoodConfig>>,
    ) -> PyResult<Self> {
        let config = config.map(|c| c.inner.clone()).unwrap_or_default();
        py.detach(move || {
            let inner = NativeNeighborhoodReader::new(path, config)
                .map_err(io_error)?;
            let acquisition =
                AcquisitionMetadata::new(acquisition_rows(inner.frame_infos()));
            Ok(Self { inner, acquisition })
        })
    }

    #[pyo3(signature = (frame, interval, scan, precursor_mz=None))]
    /// Read one source-frame/interval/scan anchor, optionally conditioned on m/z.
    fn read_at(
        &self,
        py: Python<'_>,
        frame: usize,
        interval: usize,
        scan: usize,
        precursor_mz: Option<f64>,
    ) -> PyResult<Py<PyAny>> {
        validate_mz(precursor_mz)?;
        let n = py.detach(|| {
            self.extract(
                Anchor {
                    frame,
                    interval,
                    scan,
                },
                precursor_mz,
            )
        })?;
        neighborhood(py, &n, &self.acquisition, None)
    }

    #[pyo3(signature = (anchors, *, precursor_mzs=None))]
    /// Materialize 1–1024 explicit (frame, interval, scan) anchors.
    fn read_many(
        &self,
        py: Python<'_>,
        anchors: Vec<(usize, usize, usize)>,
        precursor_mzs: Option<Vec<f64>>,
    ) -> PyResult<Vec<Py<PyAny>>> {
        let queries = queries(anchors, precursor_mzs)?;
        let results = py.detach(|| {
            queries
                .into_iter()
                .map(|(a, mz)| self.extract(a, mz))
                .collect::<PyResult<Vec<_>>>()
        })?;
        results
            .iter()
            .map(|n| neighborhood(py, n, &self.acquisition, None))
            .collect()
    }

    fn resolved_config(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        metadata(py, self.inner.resolved_config())
    }

    fn frame_infos(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let frames = PyList::empty(py);
        for frame in self.inner.frame_infos() {
            let d = PyDict::new(py);
            d.set_item("frame", frame.index())?;
            d.set_item("rt_s", frame.rt_in_seconds())?;
            d.set_item("group", frame.window_group())?;
            d.set_item("cycle_index", frame.cycle_index())?;
            d.set_item(
                "acquisition",
                metadata(
                    py,
                    &self.acquisition.for_frame(frame.index() as u64),
                )?,
            )?;
            frames.append(d)?;
        }
        Ok(frames.into_any().unbind())
    }

    /// Writes only supplied anchors; does not enumerate an entire acquisition.
    #[pyo3(signature = (path, anchors, *, precursor_mzs=None))]
    fn export(
        &self,
        py: Python<'_>,
        path: String,
        anchors: Vec<(usize, usize, usize)>,
        precursor_mzs: Option<Vec<f64>>,
    ) -> PyResult<Py<PyAny>> {
        let queries = queries(anchors, precursor_mzs)?;
        let manifest = py.detach(|| {
            let mut writer = Writer::new(
                path,
                self.inner.resolved_config(),
                "explicit",
                self.inner.frame_infos(),
            )
            .map_err(io_error)?;
            writer.set_bundle_limit(queries.len()).map_err(io_error)?;
            for (anchor, mz) in queries {
                writer
                    .write(&self.extract(anchor, mz)?, None)
                    .map_err(io_error)?;
            }
            writer.finish().map_err(io_error)
        })?;
        metadata(py, &manifest)
    }
}

/// Random access to completed observation datasets; metadata is retained in memory.
#[pyclass(name = "ParquetNeighborhoodReader", module = "tims_pyo3")]
pub(crate) struct PyParquetNeighborhoodReader {
    inner: Reader,
    acquisition: AcquisitionMetadata,
}

#[pymethods]
impl PyParquetNeighborhoodReader {
    #[new]
    fn new(py: Python<'_>, path: String) -> PyResult<Self> {
        py.detach(move || {
            let inner = Reader::new(path).map_err(io_error)?;
            let acquisition = AcquisitionMetadata::new(
                inner.acquisition().map_err(io_error)?,
            );
            Ok(Self { inner, acquisition })
        })
    }

    fn __len__(&self) -> usize {
        self.inner.manifest.bundles
    }

    /// Read a nonnegative bundle index as detached structured Python mappings.
    fn get(&self, py: Python<'_>, index: usize) -> PyResult<Py<PyAny>> {
        if index >= self.__len__() {
            return Err(PyIndexError::new_err("bundle index out of bounds"));
        }
        let result = py.detach(|| self.inner.get(index).map_err(io_error))?;
        neighborhood(
            py,
            &result.neighborhood,
            &self.acquisition,
            result.measured_precursor.as_ref(),
        )
    }

    fn __getitem__(&self, py: Python<'_>, index: usize) -> PyResult<Py<PyAny>> {
        self.get(py, index)
    }

    fn manifest(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        metadata(py, &self.inner.manifest)
    }

    fn resolved_config(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let resolved = self
            .inner
            .manifest
            .config
            .resolve(self.inner.manifest.native_scan_fwhm)
            .map_err(value_error)?;
        metadata(py, &resolved)
    }

    fn acquisition(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        metadata(
            py,
            &self
                .acquisition
                .frames
                .values()
                .flatten()
                .collect::<Vec<_>>(),
        )
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRtConsistencyConfig>()?;
    m.add_class::<PyNativeNeighborhoodConfig>()?;
    m.add_class::<PyNativeNeighborhoodReader>()?;
    m.add_class::<PyParquetNeighborhoodReader>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc};
    use timsrust::{
        centroid::neighborhood::{
            Offset, RtPeakEvidence, RtSupport, UnavailableReason,
        },
        core::{AcquisitionType, MSLevel, Mz, Spectrum},
    };

    fn synthetic() -> (Config, Vec<FrameInfo>, Neighborhood) {
        let config = Config {
            acquisition: AcquisitionPolicy::ExperimentalTrajectory,
            im: ImSizing::Scans {
                left: 3,
                right: 4,
                stride: 1,
            },
            rt_context: ContextExtent {
                before: 1,
                after: 1,
            },
            rt_consistency: Some(RtConsistencyConfig {
                minimum_support: 2,
                require_both_sides: false,
                ..RtConsistencyConfig::default()
            }),
            ..Config::default()
        };
        let q = Arc::new(QuadrupoleSettings {
            index: 8,
            scan_starts: vec![0, 4],
            scan_ends: vec![4, 6],
            isolation_windows: vec![
                IsolationWindow::new_from_bounds(
                    400.0.into(),
                    600.0.into(),
                    30.0,
                ),
                IsolationWindow::new_from_bounds(
                    450.0.into(),
                    650.0.into(),
                    32.0,
                ),
            ],
        });
        let frames: Vec<_> = [11, 12]
            .into_iter()
            .map(|index| {
                FrameInfo::new(
                    q.clone(),
                    index,
                    index as f64,
                    1.0,
                    AcquisitionType::DIAPASEF,
                    MSLevel::MS2,
                    8,
                    None,
                )
            })
            .collect();
        let p = Precursor::new(
            500.0.into(),
            1.2.into(),
            12.0.into(),
            2_u32.try_into().unwrap(),
            None,
            None,
            7,
            12_u32.try_into().unwrap(),
        );
        let response = (1_u64 << 53) + 17;
        let focal = View {
            anchor: Anchor {
                frame: 12,
                interval: 0,
                scan: 2,
            },
            rt_s: 12.0,
            inverse_mobility: 1.2,
            requested_scan_start: -1,
            requested_scan_end: 7,
            scan_start: 0,
            scan_end: 6,
            fully_covered: false,
            peak_scan_tof: vec![(3, 44)],
            peak_apex_responses: vec![response],
            peak_inverse_mobility: vec![1.19],
            spectrum: Spectrum::<Mz>::new(
                vec![response as f64],
                123,
                Some(p),
                vec![300.25.into()],
                IsolationWindow::new_from_bounds(
                    450.0.into(),
                    600.0.into(),
                    30.0,
                ),
            ),
            rt_evidence: Some(vec![RtPeakEvidence {
                native: true,
                qualified: true,
                ambiguous: false,
                support: vec![RtSupport {
                    frame: 12,
                    rt_s: 12.0,
                    scan: 3,
                    tof: 44,
                    mz: 300.25,
                    response,
                }],
                consensus_mz: 300.251,
                consensus_scan: 3.1,
                selection_score: 9.0,
            }]),
        };
        let empty = View {
            anchor: Anchor {
                frame: 11,
                ..focal.anchor
            },
            rt_s: 11.0,
            peak_scan_tof: vec![],
            peak_apex_responses: vec![],
            peak_inverse_mobility: vec![],
            spectrum: Spectrum::new(
                vec![],
                122,
                focal.spectrum.precursor().clone(),
                vec![],
                focal.spectrum.isolation_window().clone(),
            ),
            rt_evidence: Some(vec![]),
            ..focal.clone()
        };
        (
            config,
            frames,
            Neighborhood {
                focal,
                context: vec![
                    Observation::Available {
                        offset: Offset { rt: -1, im: 0 },
                        view: Box::new(empty),
                    },
                    Observation::Unavailable {
                        offset: Offset { rt: 1, im: 0 },
                        frame: None,
                        reason: UnavailableReason::RunBoundary,
                    },
                ],
            },
        )
    }

    #[test]
    fn bounded_queries_reject_bad_inputs() {
        assert!(queries(vec![], None).is_err());
        assert!(queries(vec![(0, 0, 0); MAX_BATCH + 1], None).is_err());
        assert!(queries(vec![(0, 0, 0)], Some(vec![])).is_err());
        for mz in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert!(validate_mz(Some(mz)).is_err());
        }
        assert!(queries(vec![(12, 0, 2)], Some(vec![500.0])).is_ok());
    }

    #[test]
    fn structured_mapping_preserves_measurements_and_domains() {
        let (_, frames, n) = synthetic();
        Python::initialize();
        Python::attach(|py| {
            let result = neighborhood(
                py,
                &n,
                &AcquisitionMetadata::new(acquisition_rows(&frames)),
                None,
            )
            .unwrap();
            let d = result.bind(py);
            let focal = d.get_item("focal").unwrap();
            let peak = focal.get_item("peaks").unwrap().get_item(0).unwrap();
            assert_eq!(
                peak.get_item("apex_response")
                    .unwrap()
                    .extract::<u64>()
                    .unwrap(),
                (1_u64 << 53) + 17
            );
            assert_eq!(
                peak.get_item("mz").unwrap().extract::<f64>().unwrap(),
                300.25
            );
            assert_eq!(
                peak.get_item("inverse_mobility")
                    .unwrap()
                    .extract::<f64>()
                    .unwrap(),
                1.19
            );
            let domain = focal.get_item("observation_domain").unwrap();
            assert_eq!(domain.get_item("segments").unwrap().len().unwrap(), 4);
            let context = d.get_item("context").unwrap();
            assert!(
                context
                    .get_item(0)
                    .unwrap()
                    .get_item("available")
                    .unwrap()
                    .extract::<bool>()
                    .unwrap()
            );
            assert_eq!(
                context
                    .get_item(0)
                    .unwrap()
                    .get_item("view")
                    .unwrap()
                    .get_item("peaks")
                    .unwrap()
                    .len()
                    .unwrap(),
                0
            );
            assert!(
                !context
                    .get_item(1)
                    .unwrap()
                    .get_item("available")
                    .unwrap()
                    .extract::<bool>()
                    .unwrap()
            );
        });
    }

    #[test]
    fn synthetic_parquet_roundtrip() {
        let (config, frames, n) = synthetic();
        let supplied = std::env::var_os("TIMSRUST_PYTHON_TEST_DATASET");
        let path = supplied.as_ref().map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("python-test-dataset-{}", std::process::id()))
        });
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut writer = Writer::new(
            &path,
            &config.resolve(4).unwrap(),
            "explicit",
            &frames,
        )
        .unwrap();
        writer.set_bundle_limit(1).unwrap();
        writer.write(&n, None).unwrap();
        writer.finish().unwrap();
        let reader = Reader::new(&path).unwrap();
        let result = reader.get(0).unwrap();
        assert_eq!(result.neighborhood, n);
        assert_eq!(reader.acquisition().unwrap().len(), 4);
        if supplied.is_none() {
            std::fs::remove_dir_all(path).unwrap();
        }
    }
}
