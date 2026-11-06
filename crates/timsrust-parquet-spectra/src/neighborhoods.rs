//! Versioned, streaming native-observation datasets. A manifest marks completion.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use timsrust_centroid::neighborhood::{
    Anchor, Config, Neighborhood, Observation, Offset, ResolvedConfig,
    RtPeakEvidence, UnavailableReason, View,
};
use timsrust_core::{
    FrameInfo, IsolationWindow, Mz, Precursor, Spectrum,
    io::formats::parquet::{ParquetError, ParquetReader, ParquetWriter},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parquet(#[from] ParquetError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid neighborhood dataset: {0}")]
    Invalid(String),
}
type Result<T> = std::result::Result<T, Error>;
fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}

#[derive(Serialize, Deserialize)]
struct PrecursorRecord {
    mz: f64,
    rt_s: f64,
    inverse_mobility: f64,
    scan: u32,
    frame: u32,
    charge: Option<i8>,
    intensity: Option<f64>,
    index: usize,
}
impl From<&Precursor> for PrecursorRecord {
    fn from(p: &Precursor) -> Self {
        Self {
            mz: f64::from(p.mz()),
            rt_s: f64::from(p.rt()),
            inverse_mobility: f64::from(p.im()),
            scan: u32::from(p.scan_index()),
            frame: u32::from(p.frame_index()),
            charge: p.charge().map(i8::from),
            intensity: *p.intensity(),
            index: p.index(),
        }
    }
}
fn precursor_json(p: &Precursor) -> Result<String> {
    Ok(serde_json::to_string(&PrecursorRecord::from(p))?)
}
fn read_precursor(text: &str) -> Result<Precursor> {
    let p: PrecursorRecord = serde_json::from_str(text)?;
    require(
        p.mz.is_finite()
            && p.mz > 0.0
            && p.rt_s.is_finite()
            && p.inverse_mobility.is_finite()
            && p.intensity.is_none_or(|n| n.is_finite() && n >= 0.0),
        "invalid precursor metadata",
    )?;
    Ok(Precursor::new(
        p.mz.into(),
        p.inverse_mobility.into(),
        p.rt_s.into(),
        p.scan
            .try_into()
            .map_err(|e| Error::Invalid(format!("{e}")))?,
        p.charge
            .map(|c| c.try_into().map_err(|e| Error::Invalid(format!("{e}"))))
            .transpose()?,
        p.intensity,
        p.index,
        p.frame
            .try_into()
            .map_err(|e| Error::Invalid(format!("{e}")))?,
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub complete: bool,
    pub sampling: String,
    pub config: Config,
    pub native_scan_fwhm: usize,
    pub response_definition: String,
    pub rt_unit: String,
    pub mobility_unit: String,
    pub bundles: usize,
    pub slots_per_bundle: usize,
    pub observations: usize,
    pub peaks: usize,
    pub acquisition_intervals: usize,
    pub bundle_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakRow {
    pub observation: u64,
    pub scan: u32,
    pub tof: u32,
    pub mz: f64,
    pub inverse_mobility: f64,
    pub apex_response: u64,
    pub rt_evidence_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionRow {
    pub frame: u64,
    pub interval: u64,
    pub group: u8,
    pub rt_s: f64,
    pub scan_start: u64,
    pub scan_end: u64,
    pub isolation_lower: f64,
    pub isolation_upper: f64,
    pub collision_energy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRow {
    pub bundle: u64,
    pub slot: u32,
    pub rt_offset: i32,
    pub im_offset: i32,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub frame: Option<u64>,
    pub interval: Option<u64>,
    pub scan: Option<u64>,
    pub rt_s: Option<f64>,
    pub inverse_mobility: Option<f64>,
    pub requested_scan_start: Option<i64>,
    pub requested_scan_end: Option<i64>,
    pub scan_start: Option<u64>,
    pub scan_end: Option<u64>,
    pub fully_covered: Option<bool>,
    pub spectrum_id: Option<u64>,
    pub isolation_lower: Option<f64>,
    pub isolation_upper: Option<f64>,
    pub collision_energy: Option<f64>,
    pub precursor_json: Option<String>,
    pub measured_precursor_json: Option<String>,
    pub rt_evidence_present: bool,
    pub peak_start: u64,
    pub peak_end: u64,
}

fn offsets(config: &Config) -> Vec<Offset> {
    std::iter::once(Offset { rt: 0, im: 0 })
        .chain(
            (-(config.rt_context.before as i32)
                ..=config.rt_context.after as i32)
                .flat_map(|rt| {
                    (-(config.im_context.before as i32)
                        ..=config.im_context.after as i32)
                        .map(move |im| Offset { rt, im })
                })
                .filter(|o| *o != Offset { rt: 0, im: 0 }),
        )
        .collect()
}

pub struct Writer {
    path: PathBuf,
    manifest: Manifest,
    observations: ParquetWriter<ObservationRow>,
    peaks: ParquetWriter<PeakRow>,
    failed: bool,
}

impl Writer {
    pub fn new(
        path: impl AsRef<Path>,
        resolved: &ResolvedConfig,
        sampling: &str,
        frames: &[FrameInfo],
    ) -> Result<Self> {
        require(
            matches!(sampling, "window" | "precursor" | "explicit"),
            "unknown sampling policy",
        )?;
        resolved
            .request
            .validate()
            .map_err(|e| Error::Invalid(e.to_string()))?;
        let path = path.as_ref();
        fs::create_dir(path)?;
        let mut acquisitions =
            ParquetWriter::new(path.join("acquisition.parquet"))?;
        acquisitions.set_max_batch_count(frames.len());
        let mut interval_count = 0;
        for frame in frames {
            let q = frame.quadrupole_settings();
            require(
                q.len() == q.scan_starts.len()
                    && q.len() == q.scan_ends.len()
                    && frame.rt_in_seconds().is_finite()
                    && (0..q.len()).all(|i| {
                        q.scan_starts[i] < q.scan_ends[i]
                            && f64::from(q.isolation_windows[i].lower())
                                .is_finite()
                            && f64::from(q.isolation_windows[i].upper())
                                .is_finite()
                            && q.isolation_windows[i]
                                .collision_energy()
                                .is_finite()
                    }),
                "invalid acquisition metadata",
            )?;
            let rows: Vec<_> = (0..q.len())
                .map(|i| AcquisitionRow {
                    frame: frame.index() as u64,
                    interval: i as u64,
                    group: frame.window_group(),
                    rt_s: frame.rt_in_seconds(),
                    scan_start: q.scan_starts[i] as u64,
                    scan_end: q.scan_ends[i] as u64,
                    isolation_lower: f64::from(q.isolation_windows[i].lower()),
                    isolation_upper: f64::from(q.isolation_windows[i].upper()),
                    collision_energy: q.isolation_windows[i].collision_energy(),
                })
                .collect();
            interval_count += rows.len();
            acquisitions.write_batch(rows)?;
        }
        acquisitions.finish()?;
        Ok(Self {
            path: path.into(),
            manifest: Manifest {
                schema_version: 1,
                complete: true,
                sampling: sampling.into(),
                config: resolved.request.clone(),
                native_scan_fwhm: resolved.native_scan_fwhm,
                response_definition: resolved.response_definition.into(),
                rt_unit: "seconds".into(),
                mobility_unit: "inverse_reduced_mobility".into(),
                bundles: 0,
                slots_per_bundle: offsets(&resolved.request).len(),
                observations: 0,
                peaks: 0,
                acquisition_intervals: interval_count,
                bundle_limit: None,
            },
            observations: ParquetWriter::new(
                path.join("observations.parquet"),
            )?,
            peaks: ParquetWriter::new(path.join("peaks.parquet"))?,
            failed: false,
        })
    }

    pub fn write(
        &mut self,
        bundle: &Neighborhood,
        measured_precursor: Option<&Precursor>,
    ) -> Result<()> {
        require(!self.failed, "writer failed previously")?;
        let result = self.write_bundle(bundle, measured_precursor);
        if result.is_err() {
            self.failed = true;
        }

        result
    }

    pub fn set_bundle_limit(&mut self, limit: usize) -> Result<()> {
        require(
            limit > 0 && self.manifest.bundles == 0,
            "bundle limit must be positive and set before writing",
        )?;
        self.manifest.bundle_limit = Some(limit);
        Ok(())
    }

    fn write_bundle(
        &mut self,
        bundle: &Neighborhood,
        measured_precursor: Option<&Precursor>,
    ) -> Result<()> {
        require(
            self.manifest
                .bundle_limit
                .is_none_or(|n| self.manifest.bundles < n),
            "configured bundle limit exceeded",
        )?;
        require(
            self.manifest.sampling != "precursor"
                || measured_precursor.is_some(),
            "precursor-first sampling requires measured precursor metadata",
        )?;
        let expected = offsets(&self.manifest.config);
        require(
            bundle.context.len() + 1 == expected.len(),
            "wrong context slot count",
        )?;
        for (observation, offset) in
            bundle.context.iter().zip(expected.iter().skip(1))
        {
            let actual = match observation {
                Observation::Available { offset, .. }
                | Observation::Unavailable { offset, .. } => offset,
            };
            require(
                actual == offset,
                "context slot order differs from configured geometry",
            )?;
        }
        let mut rows = Vec::with_capacity(expected.len());
        for (slot, offset) in expected.into_iter().enumerate() {
            let (view, frame, reason) = if slot == 0 {
                (Some(&bundle.focal), None, None)
            } else {
                match &bundle.context[slot - 1] {
                    Observation::Available { view, .. } => {
                        (Some(view.as_ref()), None, None)
                    },
                    Observation::Unavailable { frame, reason, .. } => {
                        (None, *frame, Some(format!("{reason:?}")))
                    },
                }
            };
            let start = self.manifest.peaks;
            let mut row = ObservationRow {
                bundle: self.manifest.bundles as u64,
                slot: slot as u32,
                rt_offset: offset.rt,
                im_offset: offset.im,
                available: view.is_some(),
                unavailable_reason: reason,
                frame: frame.map(|f| f as u64),
                interval: None,
                scan: None,
                rt_s: None,
                inverse_mobility: None,
                requested_scan_start: None,
                requested_scan_end: None,
                scan_start: None,
                scan_end: None,
                fully_covered: None,
                spectrum_id: None,
                isolation_lower: None,
                isolation_upper: None,
                collision_energy: None,
                precursor_json: None,
                measured_precursor_json: if slot == 0 {
                    measured_precursor.map(precursor_json).transpose()?
                } else {
                    None
                },
                rt_evidence_present: false,
                peak_start: start as u64,
                peak_end: start as u64,
            };
            if let Some(view) = view {
                let count = view.spectrum.len();
                require(
                    view.peak_scan_tof.len() == count
                        && view.peak_apex_responses.len() == count
                        && view.peak_inverse_mobility.len() == count
                        && view
                            .rt_evidence
                            .as_ref()
                            .is_none_or(|e| e.len() == count),
                    "misaligned peak metadata",
                )?;
                require(
                    view.rt_evidence.is_some()
                        == self.manifest.config.rt_consistency.is_some(),
                    "RT mode differs from manifest",
                )?;
                require(
                    (0..count).all(|i| {
                        view.spectrum.intensities()[i]
                            == view.peak_apex_responses[i] as f64
                            && view.peak_apex_responses[i] > 0
                            && view.peak_inverse_mobility[i].is_finite()
                            && f64::from(view.spectrum.mz_values()[i])
                                .is_finite()
                    }),
                    "invalid or inconsistent measured peak values",
                )?;
                let peaks: Vec<_> = (0..count)
                    .map(|i| {
                        Ok(PeakRow {
                            observation: (self.manifest.observations + slot)
                                as u64,
                            scan: view.peak_scan_tof[i].0,
                            tof: view.peak_scan_tof[i].1,
                            mz: f64::from(view.spectrum.mz_values()[i]),
                            inverse_mobility: view.peak_inverse_mobility[i],
                            apex_response: view.peak_apex_responses[i],
                            rt_evidence_json: view
                                .rt_evidence
                                .as_ref()
                                .map(|e| serde_json::to_string(&e[i]))
                                .transpose()?,
                        })
                    })
                    .collect::<Result<_>>()?;
                self.peaks.write_batch(peaks)?;
                self.manifest.peaks += count;
                let window = view.spectrum.isolation_window();
                row.frame = Some(view.anchor.frame as u64);
                row.interval = Some(view.anchor.interval as u64);
                row.scan = Some(view.anchor.scan as u64);
                row.rt_s = Some(view.rt_s);
                row.inverse_mobility = Some(view.inverse_mobility);
                row.requested_scan_start = Some(view.requested_scan_start);
                row.requested_scan_end = Some(view.requested_scan_end);
                row.scan_start = Some(view.scan_start as u64);
                row.scan_end = Some(view.scan_end as u64);
                row.fully_covered = Some(view.fully_covered);
                row.spectrum_id = Some(view.spectrum.index() as u64);
                row.isolation_lower = Some(f64::from(window.lower()));
                row.isolation_upper = Some(f64::from(window.upper()));
                row.collision_energy = Some(window.collision_energy());
                row.precursor_json = view
                    .spectrum
                    .precursor()
                    .as_ref()
                    .map(precursor_json)
                    .transpose()?;
                row.rt_evidence_present = view.rt_evidence.is_some();
                row.peak_end = self.manifest.peaks as u64;
            }
            rows.push(row);
        }
        self.observations.write_batch(rows)?;
        self.manifest.observations += self.manifest.slots_per_bundle;
        self.manifest.bundles += 1;
        Ok(())
    }

    pub fn finish(self) -> Result<Manifest> {
        require(!self.failed, "cannot finalize a failed export")?;
        self.observations.finish()?;
        self.peaks.finish()?;
        let temp = self.path.join("manifest.json.tmp");
        fs::write(&temp, serde_json::to_vec_pretty(&self.manifest)?)?;
        fs::rename(temp, self.path.join("manifest.json"))?;
        Ok(self.manifest)
    }
}

pub struct Reader {
    pub manifest: Manifest,
    observations: ParquetReader<ObservationRow>,
    peaks: ParquetReader<PeakRow>,
    acquisition: ParquetReader<AcquisitionRow>,
}

pub struct ExportedNeighborhood {
    pub neighborhood: Neighborhood,
    pub measured_precursor: Option<Precursor>,
}

impl Reader {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(path.join("manifest.json"))?)?;
        require(
            manifest.schema_version == 1 && manifest.complete,
            "unsupported or incomplete schema",
        )?;
        require(
            manifest.rt_unit == "seconds"
                && manifest.mobility_unit == "inverse_reduced_mobility"
                && manifest.response_definition
                    == "native_kernel_weighted_apex"
                && matches!(
                    manifest.sampling.as_str(),
                    "window" | "precursor" | "explicit"
                )
                && manifest.native_scan_fwhm > 0,
            "unsupported measurement semantics",
        )?;
        manifest
            .config
            .validate()
            .map_err(|e| Error::Invalid(e.to_string()))?;
        require(
            manifest.slots_per_bundle == offsets(&manifest.config).len(),
            "manifest geometry differs",
        )?;
        let reader = Self {
            manifest,
            observations: ParquetReader::from(
                path.join("observations.parquet"),
            )?,
            peaks: ParquetReader::from(path.join("peaks.parquet"))?,
            acquisition: ParquetReader::from(path.join("acquisition.parquet"))?,
        };
        require(
            reader.observations.shape().0 == reader.manifest.observations
                && reader.peaks.shape().0 == reader.manifest.peaks
                && reader.acquisition.shape().0
                    == reader.manifest.acquisition_intervals
                && reader
                    .manifest
                    .bundles
                    .checked_mul(reader.manifest.slots_per_bundle)
                    == Some(reader.manifest.observations),
            "manifest row counts differ",
        )?;
        Ok(reader)
    }

    pub fn acquisition(&self) -> Result<Vec<AcquisitionRow>> {
        Ok(self.acquisition.read_all()?)
    }

    pub fn get(&self, index: usize) -> Result<ExportedNeighborhood> {
        require(index < self.manifest.bundles, "bundle index out of bounds")?;
        let slots = self.manifest.slots_per_bundle;
        let rows = self
            .observations
            .read_range(index * slots..(index + 1) * slots)?;
        let expected = offsets(&self.manifest.config);
        let mut views = Vec::with_capacity(slots);
        let mut measured_precursor = None;
        for (slot, row) in rows.into_iter().enumerate() {
            require(
                row.bundle == index as u64
                    && row.slot == slot as u32
                    && (row.rt_offset, row.im_offset)
                        == (expected[slot].rt, expected[slot].im),
                "observation identity or offsets differ",
            )?;
            require(
                row.peak_start <= row.peak_end
                    && row.peak_end <= self.manifest.peaks as u64,
                "invalid peak range",
            )?;
            if slot == 0 {
                measured_precursor = row
                    .measured_precursor_json
                    .as_deref()
                    .map(read_precursor)
                    .transpose()?;
            }
            if !row.available {
                require(
                    slot != 0 && row.peak_start == row.peak_end,
                    "missing focal or nonempty missing observation",
                )?;
                let reason = match row.unavailable_reason.as_deref() {
                    Some("RunBoundary") => UnavailableReason::RunBoundary,
                    Some("AcquisitionChanged") => {
                        UnavailableReason::AcquisitionChanged
                    },
                    Some("CenterOutsideIsolation") => {
                        UnavailableReason::CenterOutsideIsolation
                    },
                    Some("TimeGap") => UnavailableReason::TimeGap,
                    _ => {
                        return Err(Error::Invalid(
                            "unknown unavailable reason".into(),
                        ));
                    },
                };
                views.push(Observation::Unavailable {
                    offset: expected[slot],
                    frame: row.frame.map(|f| f as usize),
                    reason,
                });
                continue;
            }
            require(
                row.peak_start <= row.peak_end
                    && row.peak_end <= self.manifest.peaks as u64,
                "invalid peak range",
            )?;
            let peaks = self
                .peaks
                .read_range(row.peak_start as usize..row.peak_end as usize)?;
            require(
                peaks
                    .iter()
                    .all(|p| p.observation == (index * slots + slot) as u64),
                "cross-observation peak reference",
            )?;
            require(
                row.unavailable_reason.is_none()
                    && row.rt_evidence_present
                        == self.manifest.config.rt_consistency.is_some()
                    && peaks.iter().all(|p| {
                        p.apex_response > 0
                            && p.mz.is_finite()
                            && p.mz > 0.0
                            && p.inverse_mobility.is_finite()
                            && p.inverse_mobility > 0.0
                    }),
                "invalid available observation",
            )?;
            macro_rules! field {
                ($name:ident) => {
                    row.$name.ok_or_else(|| {
                        Error::Invalid(
                            concat!("missing ", stringify!($name)).into(),
                        )
                    })?
                };
            }
            let precursor = row
                .precursor_json
                .as_deref()
                .map(read_precursor)
                .transpose()?;
            let evidence = if row.rt_evidence_present {
                Some(
                    peaks
                        .iter()
                        .map(|p| {
                            p.rt_evidence_json
                                .as_deref()
                                .ok_or_else(|| {
                                    Error::Invalid(
                                        "missing RT provenance".into(),
                                    )
                                })
                                .and_then(|s| {
                                    Ok(serde_json::from_str::<RtPeakEvidence>(
                                        s,
                                    )?)
                                })
                        })
                        .collect::<Result<Vec<_>>>()?,
                )
            } else {
                require(
                    peaks.iter().all(|p| p.rt_evidence_json.is_none()),
                    "unexpected RT provenance",
                )?;
                None
            };
            let view = View {
                anchor: Anchor {
                    frame: field!(frame) as usize,
                    interval: field!(interval) as usize,
                    scan: field!(scan) as usize,
                },
                rt_s: field!(rt_s),
                inverse_mobility: field!(inverse_mobility),
                requested_scan_start: field!(requested_scan_start),
                requested_scan_end: field!(requested_scan_end),
                scan_start: field!(scan_start) as usize,
                scan_end: field!(scan_end) as usize,
                fully_covered: field!(fully_covered),
                peak_scan_tof: peaks.iter().map(|p| (p.scan, p.tof)).collect(),
                peak_apex_responses: peaks
                    .iter()
                    .map(|p| p.apex_response)
                    .collect(),
                peak_inverse_mobility: peaks
                    .iter()
                    .map(|p| p.inverse_mobility)
                    .collect(),
                spectrum: Spectrum::new(
                    peaks.iter().map(|p| p.apex_response as f64).collect(),
                    field!(spectrum_id) as usize,
                    precursor,
                    peaks.iter().map(|p| Mz::from(p.mz)).collect(),
                    IsolationWindow::new_from_bounds(
                        field!(isolation_lower).into(),
                        field!(isolation_upper).into(),
                        field!(collision_energy),
                    ),
                ),
                rt_evidence: evidence,
            };
            views.push(Observation::Available {
                offset: expected[slot],
                view: Box::new(view),
            });
        }
        let focal = match views.remove(0) {
            Observation::Available { view, .. } => *view,
            _ => return Err(Error::Invalid("missing focal".into())),
        };
        Ok(ExportedNeighborhood {
            neighborhood: Neighborhood {
                focal,
                context: views,
            },
            measured_precursor,
        })
    }
}

#[cfg(test)]
mod tests;
