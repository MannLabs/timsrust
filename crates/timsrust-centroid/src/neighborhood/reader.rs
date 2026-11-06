use std::collections::{BTreeMap, BTreeSet};

use timsrust_core::{
    FrameIndex, FrameInfo, MSLevel, Mz, Precursor, Rt, ScanIndex, Spectrum,
};

use super::{BoundaryPolicy, Config, NativeSource, ResolvedConfig};
use crate::{Peak, TimsError, TimsResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub frame: usize,
    pub interval: usize,
    pub scan: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offset {
    /// Same-group observation steps, not frame arithmetic or seconds.
    pub rt: i32,
    /// Increasing scan direction, not increasing physical inverse mobility.
    pub im: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub anchor: Anchor,
    pub rt_s: f64,
    pub inverse_mobility: f64,
    pub requested_scan_start: i64,
    pub requested_scan_end: i64,
    pub scan_start: usize,
    pub scan_end: usize,
    /// IM coverage only; RT context retains its actual, potentially uneven times.
    pub fully_covered: bool,
    /// Original centroid coordinates, in the same order as the spectrum peaks.
    pub peak_scan_tof: Vec<(u32, u32)>,
    /// Exact integer measurements, aligned with the spectrum peaks.
    pub peak_apex_responses: Vec<u64>,
    pub peak_inverse_mobility: Vec<f64>,
    /// Native-kernel filtered-apex responses; not integrated raw volume.
    /// Isolation is a selected-source summary; use acquisition rows and
    /// `observation_domain` for physical scan-wise transmission metadata.
    pub spectrum: Spectrum<Mz>,
    /// Aligned with selected peaks; populated only by opt-in RT consistency.
    pub rt_evidence: Option<Vec<super::RtPeakEvidence>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    RunBoundary,
    AcquisitionChanged,
    CenterOutsideIsolation,
    TimeGap,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Observation {
    Available {
        offset: Offset,
        view: Box<View>,
    },
    Unavailable {
        offset: Offset,
        frame: Option<usize>,
        reason: UnavailableReason,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Neighborhood {
    pub focal: View,
    pub context: Vec<Observation>,
}

impl Neighborhood {
    fn is_complete(&self) -> bool {
        self.focal.fully_covered && self.context.iter().all(|o|
            matches!(o, Observation::Available { view, .. } if view.fully_covered))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IterationStats {
    pub frames_loaded: usize,
    pub peak_frames_high_water: usize,
    pub anchors_yielded: usize,
    pub anchors_skipped_incomplete: usize,
}

pub struct NeighborhoodReader<S> {
    pub(super) source: S,
    resolved: ResolvedConfig,
    pub(super) frames: Vec<FrameInfo>,
    pub(super) groups: BTreeMap<u8, Vec<usize>>,
    pub(super) positions: Vec<usize>,
    pub(super) rt_cadences: BTreeMap<u8, f64>,
}

fn invalid(message: impl Into<String>) -> TimsError {
    TimsError::new(message)
}

impl<S: NativeSource> NeighborhoodReader<S> {
    /// Requested IM domain, including untransmitted/missing tails as masked cells.
    pub fn observation_domain(
        &self,
        view: &View,
    ) -> TimsResult<super::ObservationDomain> {
        let info = self
            .frames
            .iter()
            .find(|f| f.index() == view.anchor.frame)
            .ok_or_else(|| {
                invalid("view frame does not belong to this reader")
            })?;
        let precursor = view
            .spectrum
            .precursor()
            .as_ref()
            .ok_or_else(|| invalid("view lacks precursor mass"))?;
        super::ObservationDomain::new(
            info.quadrupole_settings(),
            view.requested_scan_start,
            view.requested_scan_end,
            f64::from(precursor.mz()),
        )
    }

    pub fn new(source: S, config: Config) -> TimsResult<Self> {
        let resolved = config.resolve(source.scan_fwhm())?;
        let mut frames = source.frame_infos()?;
        frames.retain(|f| f.ms_level() == MSLevel::MS2);
        frames.sort_by(|a, b| {
            a.rt_in_seconds()
                .total_cmp(&b.rt_in_seconds())
                .then(a.index().cmp(&b.index()))
        });
        let mut ids = BTreeSet::new();
        let mut groups = BTreeMap::<u8, Vec<usize>>::new();
        let mut positions = Vec::with_capacity(frames.len());
        for (i, f) in frames.iter().enumerate() {
            let q = f.quadrupole_settings();
            if !ids.insert(f.index())
                || f.index() >= u32::MAX as usize
                || !f.rt_in_seconds().is_finite()
                || f.rt_in_seconds() < 0.0
                || f.window_group() == 0
                || q.is_empty()
                || q.scan_starts.len() != q.len()
                || q.scan_ends.len() != q.len()
                || q.scan_starts
                    .iter()
                    .zip(&q.scan_ends)
                    .any(|(&a, &b)| a >= b || b >= u32::MAX as usize)
                || q.scan_ends
                    .windows(2)
                    .zip(q.scan_starts.iter().skip(1))
                    .any(|(p, &s)| p[0] > s)
                || q.isolation_windows.iter().any(|w| {
                    let (a, b) = (f64::from(w.lower()), f64::from(w.upper()));
                    !a.is_finite()
                        || !b.is_finite()
                        || a <= 0.0
                        || a >= b
                        || !w.collision_energy().is_finite()
                })
            {
                return Err(invalid(format!(
                    "invalid native acquisition metadata for frame {}",
                    f.index()
                )));
            }
            let group = groups.entry(f.window_group()).or_default();
            if group.last().is_some_and(|&j| {
                frames[j].rt_in_seconds() >= f.rt_in_seconds()
            }) {
                return Err(invalid("non-increasing same-group RT"));
            }
            positions.push(group.len());
            group.push(i);
        }
        if frames.is_empty() {
            return Err(invalid(
                "native neighborhoods require MS2 acquisition intervals",
            ));
        }
        let rt_cadences = groups
            .iter()
            .filter_map(|(&group, indices)| {
                let mut deltas: Vec<_> = indices
                    .windows(2)
                    .map(|p| {
                        frames[p[1]].rt_in_seconds()
                            - frames[p[0]].rt_in_seconds()
                    })
                    .collect();
                deltas.sort_by(f64::total_cmp);
                (!deltas.is_empty()).then(|| (group, deltas[deltas.len() / 2]))
            })
            .collect();
        let reader = Self {
            source,
            resolved,
            frames,
            groups,
            positions,
            rt_cadences,
        };
        Ok(reader)
    }

    pub fn resolved_config(&self) -> &ResolvedConfig {
        &self.resolved
    }

    pub fn frame_infos(&self) -> &[FrameInfo] {
        &self.frames
    }

    pub fn neighborhoods(&self) -> NeighborhoodIter<'_, S> {
        self.iter(true)
    }

    /// Extracts an explicit acquisition coordinate without snapping to the sampling grid.
    pub fn neighborhood_at(&self, anchor: Anchor) -> TimsResult<Neighborhood> {
        self.extract_at(&mut self.iter(true), anchor, None)
    }

    /// Extracts a measured-precursor transmission footprint without snapping.
    pub fn neighborhood_for_precursor(
        &self,
        anchor: Anchor,
        mz: f64,
    ) -> TimsResult<Neighborhood> {
        self.extract_at(&mut self.iter(true), anchor, Some(mz))
    }

    /// Extracts explicit coordinates with a shared bounded cache, stopping after one error.
    pub fn neighborhoods_at<'a>(
        &'a self,
        anchors: impl Iterator<Item = Anchor> + 'a,
    ) -> impl Iterator<Item = TimsResult<Neighborhood>> + 'a {
        anchors.scan((self.iter(true), false), move |(iter, failed), anchor| {
            if *failed {
                return None;
            }

            let result = self.extract_at(iter, anchor, None);
            *failed = result.is_err();
            Some(result)
        })
    }

    /// Retains caller metadata and a shared cache, propagating input errors.
    pub fn neighborhoods_with<'a, T: 'a>(
        &'a self,
        anchors: impl Iterator<Item = TimsResult<(Anchor, T)>> + 'a,
    ) -> impl Iterator<Item = TimsResult<(Neighborhood, T)>> + 'a {
        self.neighborhoods_with_reference(anchors, |_| None)
    }

    pub fn neighborhoods_with_precursors<'a>(
        &'a self,
        anchors: impl Iterator<Item = TimsResult<(Anchor, Precursor)>> + 'a,
    ) -> impl Iterator<Item = TimsResult<(Neighborhood, Precursor)>> + 'a {
        self.neighborhoods_with_reference(anchors, |p| Some(f64::from(p.mz())))
    }

    fn neighborhoods_with_reference<'a, T: 'a>(
        &'a self,
        anchors: impl Iterator<Item = TimsResult<(Anchor, T)>> + 'a,
        reference: impl Fn(&T) -> Option<f64> + 'a,
    ) -> impl Iterator<Item = TimsResult<(Neighborhood, T)>> + 'a {
        anchors.scan((self.iter(true), false), move |(iter, failed), item| {
            if *failed {
                return None;
            }
            let result = item.and_then(|(anchor, metadata)| {
                self.extract_at(iter, anchor, reference(&metadata))
                    .map(|bundle| (bundle, metadata))
            });
            *failed = result.is_err();
            Some(result)
        })
    }

    fn extract_at(
        &self,
        iter: &mut NeighborhoodIter<'_, S>,
        anchor: Anchor,
        precursor_mz: Option<f64>,
    ) -> TimsResult<Neighborhood> {
        let frame = self
            .frames
            .iter()
            .position(|f| f.index() == anchor.frame)
            .ok_or_else(|| invalid("unknown native focal frame"))?;
        let q = self.frames[frame].quadrupole_settings();
        if anchor.interval >= q.len()
            || !(q.scan_starts[anchor.interval]..q.scan_ends[anchor.interval])
                .contains(&anchor.scan)
        {
            return Err(invalid(
                "native focal coordinate lies outside its isolation interval",
            ));
        }
        if precursor_mz.is_some_and(|mz| {
            !mz.is_finite()
                || mz <= 0.0
                || mz < f64::from(q.isolation_windows[anchor.interval].lower())
                || mz >= f64::from(q.isolation_windows[anchor.interval].upper())
        }) {
            return Err(invalid(
                "precursor mass lies outside its acquisition interval",
            ));
        }
        let result = iter.extract(Anchor { frame, ..anchor }, precursor_mz)?;
        if self.resolved.request.boundary == BoundaryPolicy::RequireComplete
            && !result.is_complete()
        {
            return Err(invalid(
                "explicit native focal coordinate has incomplete context",
            ));
        }
        Ok(result)
    }

    /// Projects the focal spectrum. Native mode skips unused context frames;
    /// opt-in RT consistency loads context before selecting the focal peaks.
    pub fn pseudospectra(
        &self,
    ) -> impl Iterator<Item = TimsResult<Spectrum<Mz>>> + '_ {
        self.iter(false)
            .map(|result| result.map(|n| n.focal.spectrum))
    }

    pub(super) fn iter(&self, context: bool) -> NeighborhoodIter<'_, S> {
        NeighborhoodIter {
            reader: self,
            frame: 0,
            interval: 0,
            scan: None,
            cache: BTreeMap::new(),
            rt_metadata: BTreeMap::new(),
            stats: IterationStats::default(),
            context,
            failed: false,
            reference_mz: 0.0,
        }
    }
}

pub struct NeighborhoodIter<'a, S> {
    reader: &'a NeighborhoodReader<S>,
    frame: usize,
    interval: usize,
    scan: Option<usize>,
    pub(super) cache: BTreeMap<usize, Vec<Peak>>,
    pub(super) rt_metadata:
        BTreeMap<usize, super::rt_consistency::FrameMetadata>,
    pub(super) stats: IterationStats,
    context: bool,
    failed: bool,
    reference_mz: f64,
}

impl<S: NativeSource> NeighborhoodIter<'_, S> {
    pub fn stats(&self) -> IterationStats {
        self.stats
    }

    pub(super) fn next_anchor(&mut self) -> Option<Anchor> {
        while self.frame < self.reader.frames.len() {
            let info = &self.reader.frames[self.frame];
            let q = info.quadrupole_settings();
            if self.interval == q.len() {
                self.frame += 1;
                self.interval = 0;
                self.scan = None;
                continue;
            }
            let scan = self.scan.unwrap_or(q.scan_starts[self.interval]);
            if scan >= q.scan_ends[self.interval] {
                self.interval += 1;
                self.scan = None;
                continue;
            }
            self.scan = Some(scan + self.reader.resolved.anchor_stride_scans);
            return Some(Anchor {
                frame: self.frame,
                interval: self.interval,
                scan,
            });
        }
        None
    }

    pub(super) fn context_frame(
        &self,
        anchor: Anchor,
        offset: i32,
    ) -> Result<usize, (UnavailableReason, Option<usize>)> {
        let focal = &self.reader.frames[anchor.frame];
        let group = &self.reader.groups[&focal.window_group()];
        let position = self.reader.positions[anchor.frame]
            .checked_add_signed(offset as isize)
            .and_then(|p| group.get(p))
            .copied()
            .ok_or((UnavailableReason::RunBoundary, None))?;
        let info = &self.reader.frames[position];
        let compatible = |other: &FrameInfo| {
            other.acquisition_type() == focal.acquisition_type()
                && if self.reader.resolved.request.acquisition
                    == super::AcquisitionPolicy::ExperimentalTrajectory
                {
                    super::transmission::compatible(
                        focal.quadrupole_settings(),
                        other.quadrupole_settings(),
                        self.reference_mz,
                    )
                } else {
                    other.quadrupole_settings() == focal.quadrupole_settings()
                }
        };
        if offset != 0 && self.reader.resolved.request.rt_consistency.is_some()
        {
            let from = self.reader.positions[anchor.frame];
            let to = self.reader.positions[position];
            let cadence = self.reader.rt_cadences[&focal.window_group()];
            if (from.min(to)..from.max(to)).any(|p| {
                let a = &self.reader.frames[group[p]];
                let b = &self.reader.frames[group[p + 1]];
                !compatible(a) || !compatible(b)
            }) {
                return Err((
                    UnavailableReason::AcquisitionChanged,
                    Some(info.index()),
                ));
            }
            if (from.min(to)..from.max(to)).any(|p| {
                self.reader.frames[group[p + 1]].rt_in_seconds()
                    - self.reader.frames[group[p]].rt_in_seconds()
                    > 1.5 * cadence
            }) {
                return Err((UnavailableReason::TimeGap, Some(info.index())));
            }
        }
        if !compatible(info) {
            return Err((
                UnavailableReason::AcquisitionChanged,
                Some(info.index()),
            ));
        }
        Ok(position)
    }

    pub(super) fn view(
        &mut self,
        mut anchor: Anchor,
    ) -> TimsResult<Option<View>> {
        let resolved = &self.reader.resolved;
        let info = &self.reader.frames[anchor.frame];
        let q = info.quadrupole_settings();
        let shared = resolved.request.acquisition
            != super::AcquisitionPolicy::SingleInterval;
        if resolved.request.acquisition
            == super::AcquisitionPolicy::ExperimentalTrajectory
        {
            anchor.interval = match q.get_isolation_window_index(anchor.scan) {
                Some(i) => i,
                None => return Ok(None),
            };
        }
        let (start, end) = if shared {
            match super::transmission::span_for(
                q,
                anchor.interval,
                self.reference_mz,
                resolved.request.acquisition,
            ) {
                Some(span) => span,
                None => return Ok(None),
            }
        } else {
            (q.scan_starts[anchor.interval], q.scan_ends[anchor.interval])
        };
        if !(start..end).contains(&anchor.scan) {
            return Ok(None);
        }
        if shared {
            anchor.interval =
                q.get_isolation_window_index(anchor.scan).ok_or_else(|| {
                    invalid("transmission span has an acquisition gap")
                })?;
        }
        if let std::collections::btree_map::Entry::Vacant(entry) =
            self.cache.entry(anchor.frame)
        {
            let peaks = if resolved.request.rt_consistency.is_some() || shared {
                let (peaks, metadata) = super::rt_consistency::load_candidates(
                    &self.reader.source,
                    info,
                    resolved.request.acquisition,
                    resolved.request.rt_consistency.is_some(),
                )?;
                self.rt_metadata.insert(anchor.frame, metadata);
                peaks
            } else {
                super::source::validated_peaks(&self.reader.source, info)?
            };
            entry.insert(peaks);
            self.stats.frames_loaded += 1;
            self.stats.peak_frames_high_water =
                self.stats.peak_frames_high_water.max(self.cache.len());
        }
        let requested_start =
            anchor.scan as i64 - resolved.im_left_scans as i64;
        let requested_end =
            anchor.scan as i64 + resolved.im_right_scans as i64 + 1;
        let scan_start = requested_start.max(start as i64) as usize;
        let scan_end = requested_end.min(end as i64) as usize;
        let peaks = &self.cache[&anchor.frame];
        let lower = peaks.partition_point(|p| (p.scan as usize) < scan_start);
        let upper = peaks.partition_point(|p| (p.scan as usize) < scan_end);
        let mut selected: Vec<_> = peaks[lower..upper]
            .iter()
            .filter(|p| {
                !shared
                    || self.rt_metadata[&anchor.frame]
                        .transmits(&(p.scan, p.tof), self.reference_mz)
            })
            .collect();
        selected.sort_by(|a, b| {
            b.apex_intensity
                .cmp(&a.apex_intensity)
                .then(a.tof.cmp(&b.tof))
                .then(a.scan.cmp(&b.scan))
        });
        if resolved.request.peak_cap != 0
            && resolved.request.rt_consistency.is_none()
        {
            selected.truncate(resolved.request.peak_cap);
        }
        let mut measured = selected
            .into_iter()
            .map(|p| self.reader.source.mz(p.tof).map(|mz| (p, mz)))
            .collect::<TimsResult<Vec<_>>>()?;
        if measured.iter().any(|(_, mz)| !mz.is_finite() || *mz <= 0.0) {
            return Err(invalid("invalid native mass calibration"));
        }
        measured.sort_by(|(a, am), (b, bm)| {
            am.total_cmp(bm)
                .then(a.scan.cmp(&b.scan))
                .then(a.tof.cmp(&b.tof))
        });
        let inverse_mobility = self.reader.source.mobility(anchor.scan)?;
        if !inverse_mobility.is_finite() || inverse_mobility <= 0.0 {
            return Err(invalid("invalid native mobility calibration"));
        }
        let id = usize::try_from(
            ((info.index() as u64) << 32) | anchor.scan as u64,
        )
        .map_err(|_| {
            invalid("native neighborhood IDs require a 64-bit spectrum index")
        })?;
        let precursor = Precursor::new(
            if shared {
                Mz::from(self.reference_mz)
            } else {
                q.isolation_windows[anchor.interval].center()
            },
            inverse_mobility.into(),
            Rt::from(info.rt_in_seconds()),
            ScanIndex::try_from(anchor.scan)
                .map_err(|e| invalid(e.to_string()))?,
            None,
            None,
            id,
            FrameIndex::try_from(info.index())
                .map_err(|e| invalid(e.to_string()))?,
        );
        let isolation = if shared {
            let mut lower =
                f64::from(q.isolation_windows[anchor.interval].lower());
            let mut upper =
                f64::from(q.isolation_windows[anchor.interval].upper());
            let footprints = self.rt_metadata[&anchor.frame]
                .footprints
                .as_ref()
                .expect("shared transmission footprints were loaded");
            for (p, _) in &measured {
                let f = &footprints[&(p.scan, p.tof)];
                lower = lower.max(f.lower_mz);
                upper = upper.min(f.upper_mz);
            }
            timsrust_core::IsolationWindow::new_from_bounds(
                Mz::from(lower),
                Mz::from(upper),
                q.isolation_windows[anchor.interval].collision_energy(),
            )
        } else {
            q.isolation_windows[anchor.interval].clone()
        };
        let spectrum = Spectrum::new(
            measured
                .iter()
                .map(|(p, _)| p.apex_intensity as f64)
                .collect(),
            id,
            Some(precursor),
            measured.iter().map(|(_, mz)| Mz::from(*mz)).collect(),
            isolation,
        );
        Ok(Some(View {
            anchor: Anchor {
                frame: info.index(),
                ..anchor
            },
            rt_s: info.rt_in_seconds(),
            inverse_mobility,
            requested_scan_start: requested_start,
            requested_scan_end: requested_end,
            scan_start,
            scan_end,
            fully_covered: requested_start >= start as i64
                && requested_end <= end as i64,
            peak_scan_tof: measured
                .iter()
                .map(|(p, _)| (p.scan, p.tof))
                .collect(),
            peak_apex_responses: measured
                .iter()
                .map(|(p, _)| p.apex_intensity)
                .collect(),
            peak_inverse_mobility: measured
                .iter()
                .map(|(p, _)| self.reader.source.mobility(p.scan as usize))
                .collect::<TimsResult<_>>()?,
            spectrum,
            rt_evidence: None,
        }))
    }

    fn extract(
        &mut self,
        anchor: Anchor,
        precursor_mz: Option<f64>,
    ) -> TimsResult<Neighborhood> {
        self.reference_mz = precursor_mz.unwrap_or_else(|| {
            f64::from(
                self.reader.frames[anchor.frame]
                    .quadrupole_settings()
                    .isolation_windows[anchor.interval]
                    .center(),
            )
        });
        let mut result = self.extract_unranked(anchor)?;
        if let Some(config) = &self.reader.resolved.request.rt_consistency {
            super::rt_consistency::select(
                &mut result,
                config,
                self.reader.resolved.request.peak_cap,
                self.reader.resolved.native_scan_fwhm / 2,
                &self.rt_metadata,
                &self.reader.frames,
                self.reference_mz,
            )?;
            if self.reader.resolved.request.acquisition
                != super::AcquisitionPolicy::SingleInterval
            {
                self.refresh_isolation_summaries(&mut result)?;
            }
        }
        Ok(result)
    }

    fn refresh_isolation_summaries(
        &self,
        neighborhood: &mut Neighborhood,
    ) -> TimsResult<()> {
        let views = std::iter::once(&mut neighborhood.focal).chain(
            neighborhood.context.iter_mut().filter_map(|observation| {
                match observation {
                    Observation::Available { view, .. } => Some(view.as_mut()),
                    Observation::Unavailable { .. } => None,
                }
            }),
        );
        for view in views {
            let index = self
                .reader
                .frames
                .iter()
                .position(|f| f.index() == view.anchor.frame)
                .ok_or_else(|| {
                    invalid("selected view lacks acquisition metadata")
                })?;
            let window = &self.reader.frames[index]
                .quadrupole_settings()
                .isolation_windows[view.anchor.interval];
            let footprints = self
                .rt_metadata
                .get(&index)
                .and_then(|m| m.footprints.as_ref())
                .ok_or_else(|| {
                    invalid("selected view lacks transmission footprints")
                })?;
            let mut lower = f64::from(window.lower());
            let mut upper = f64::from(window.upper());
            for id in &view.peak_scan_tof {
                let footprint = footprints.get(id).ok_or_else(|| {
                    invalid("selected source lacks a transmission footprint")
                })?;
                lower = lower.max(footprint.lower_mz);
                upper = upper.min(footprint.upper_mz);
            }
            if lower >= upper {
                return Err(invalid(
                    "selected sources have no common transmitted mass interval",
                ));
            }
            // Rejected candidates must not constrain the retained measurement summary.
            view.spectrum = Spectrum::new(
                view.spectrum.intensities().to_vec(),
                view.spectrum.index(),
                view.spectrum.precursor().clone(),
                view.spectrum.mz_values().to_vec(),
                timsrust_core::IsolationWindow::new_from_bounds(
                    lower.into(),
                    upper.into(),
                    window.collision_energy(),
                ),
            );
        }
        Ok(())
    }

    fn extract_unranked(&mut self, anchor: Anchor) -> TimsResult<Neighborhood> {
        let include_context = self.context
            || self.reader.resolved.request.rt_consistency.is_some()
            || self.reader.resolved.request.boundary
                == BoundaryPolicy::RequireComplete;
        let rt = self.reader.resolved.request.rt_context;
        let im = self.reader.resolved.request.im_context;
        let offsets: Vec<_> = if include_context {
            (-(rt.before as i32)..=rt.after as i32)
                .flat_map(|rt| {
                    (-(im.before as i32)..=im.after as i32)
                        .map(move |im| Offset { rt, im })
                })
                .filter(|o| *o != Offset { rt: 0, im: 0 })
                .collect()
        } else {
            Vec::new()
        };
        let needed: BTreeSet<_> = std::iter::once(anchor.frame)
            .chain(
                offsets
                    .iter()
                    .filter_map(|o| self.context_frame(anchor, o.rt).ok()),
            )
            .collect();
        self.cache.retain(|frame, _| needed.contains(frame));
        self.rt_metadata.retain(|frame, _| needed.contains(frame));
        let focal = self
            .view(anchor)?
            .ok_or_else(|| invalid("enumerated focal anchor is unsampled"))?;
        let mut context = Vec::with_capacity(offsets.len());
        for offset in offsets {
            let frame = match self.context_frame(anchor, offset.rt) {
                Ok(frame) => frame,
                Err((reason, frame)) => {
                    context.push(Observation::Unavailable {
                        offset,
                        frame,
                        reason,
                    });
                    continue;
                },
            };
            let scan = anchor.scan as i64
                + offset.im as i64
                    * self.reader.resolved.im_context_step_scans as i64;
            let view = if scan >= 0 && scan < u32::MAX as i64 {
                self.view(Anchor {
                    frame,
                    interval: anchor.interval,
                    scan: scan as usize,
                })?
            } else {
                None
            };
            context.push(match view {
                Some(view) => Observation::Available {
                    offset,
                    view: Box::new(view),
                },
                None => Observation::Unavailable {
                    offset,
                    frame: Some(self.reader.frames[frame].index()),
                    reason: UnavailableReason::CenterOutsideIsolation,
                },
            });
        }
        Ok(Neighborhood { focal, context })
    }
}

impl<S: NativeSource> Iterator for NeighborhoodIter<'_, S> {
    type Item = TimsResult<Neighborhood>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        while let Some(anchor) = self.next_anchor() {
            match self.extract(anchor, None) {
                Ok(mut result) => {
                    if self.reader.resolved.request.boundary
                        == BoundaryPolicy::RequireComplete
                        && !result.is_complete()
                    {
                        self.stats.anchors_skipped_incomplete += 1;
                        continue;
                    }
                    if !self.context {
                        result.context.clear();
                    }
                    self.stats.anchors_yielded += 1;
                    return Some(Ok(result));
                },
                Err(error) => {
                    self.failed = true;
                    return Some(Err(error));
                },
            }
        }
        None
    }
}
