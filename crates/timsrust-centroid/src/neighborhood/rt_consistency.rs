use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use timsrust_core::{FrameInfo, Spectrum};

use super::{
    NativeFrame, NativeSource, Neighborhood, Observation, Offset, View,
};
use crate::{Peak, TimsError, TimsResult};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RtSelectionPolicy {
    SupportWeighted,
    /// RT qualifies candidates; focal response alone determines their rank.
    FocalResponse,
    /// Protect native top-N membership; fill spare capacity by support score.
    #[default]
    NativePreserving,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RtConsistencyConfig {
    pub mz_ppm: f64,
    /// Includes this frame; each distinct acquired frame counts at most once.
    pub minimum_support: usize,
    pub require_both_sides: bool,
    pub selection: RtSelectionPolicy,
}

impl Default for RtConsistencyConfig {
    fn default() -> Self {
        Self {
            mz_ppm: 10.0,
            minimum_support: 3,
            require_both_sides: true,
            selection: RtSelectionPolicy::default(),
        }
    }
}

impl RtConsistencyConfig {
    pub fn validate(&self) -> TimsResult<()> {
        if !self.mz_ppm.is_finite()
            || self.mz_ppm <= 0.0
            || self.mz_ppm > 100.0
            || !(2..=65).contains(&self.minimum_support)
        {
            return Err(TimsError::new(
                "invalid RT consistency tolerance/support",
            ));
        }
        Ok(())
    }
}

/// Source-level candidates, before acquisition slicing or Top-N selection.
/// `weak` must have disjoint raw support from native and other weak peaks.
pub struct RtCandidateFrame {
    pub native: Vec<Peak>,
    pub weak: Vec<Peak>,
    /// Raw-verified apices under the source's acquisition policy.
    pub eligible: BTreeSet<(u32, u32)>,
    pub scan_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RtSupport {
    pub frame: usize,
    pub rt_s: f64,
    pub scan: u32,
    pub tof: u32,
    pub mz: f64,
    /// Exact native-kernel response in the support source frame.
    pub response: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RtPeakEvidence {
    pub native: bool,
    pub qualified: bool,
    pub ambiguous: bool,
    pub support: Vec<RtSupport>,
    /// Local consensus coordinates, never substituted for measured coordinates.
    pub consensus_mz: f64,
    pub consensus_scan: f64,
    /// Policy-specific selection score; never substituted for measured response.
    pub selection_score: f64,
}

pub(super) struct FrameMetadata {
    native: BTreeSet<(u32, u32)>,
    eligible: BTreeSet<(u32, u32)>,
    responses: BTreeMap<(u32, u32), u64>,
    matching_rows: Vec<Vec<RtSupport>>,
    pub(super) footprints:
        Option<BTreeMap<(u32, u32), super::MassTransmissionFootprint>>,
    policy: super::AcquisitionPolicy,
}

impl FrameMetadata {
    pub(super) fn transmits(&self, id: &(u32, u32), mz: f64) -> bool {
        self.footprints
            .as_ref()
            .is_none_or(|f| f.get(id).is_some_and(|p| p.contains(mz)))
    }

    fn domain(&self, view: &View, info: &FrameInfo, mz: f64) -> Vec<RtSupport> {
        if self.footprints.is_none() {
            return self.matching_rows[view.anchor.interval].clone();
        }
        let q = info.quadrupole_settings();
        let (start, end) = super::transmission::span_for(
            q,
            view.anchor.interval,
            mz,
            self.policy,
        )
        .expect("validated transmission view");
        let mut points: Vec<_> = self
            .matching_rows
            .iter()
            .flatten()
            .filter(|p| {
                start <= p.scan as usize
                    && (p.scan as usize) < end
                    && self.transmits(&(p.scan, p.tof), mz)
            })
            .cloned()
            .collect();
        points.sort_by(|a, b| {
            a.mz.total_cmp(&b.mz)
                .then(a.scan.cmp(&b.scan))
                .then(a.tof.cmp(&b.tof))
        });
        points
    }
}

pub(super) fn load_candidates(
    source: &impl NativeSource,
    info: &FrameInfo,
    policy: super::AcquisitionPolicy,
    include_weak: bool,
) -> TimsResult<(Vec<Peak>, FrameMetadata)> {
    let (frame, footprints) = match policy {
        super::AcquisitionPolicy::SharedTransmission => {
            let loaded = source
                .load_transmission_candidates(info.index(), include_weak)?;
            if loaded
                .footprints
                .values()
                .any(|f| !f.collision_energy.is_finite())
            {
                return Err(TimsError::new(
                    "invalid transmission collision energy",
                ));
            }
            (
                loaded.candidates,
                Some(
                    loaded
                        .footprints
                        .into_iter()
                        .map(|(id, f)| (id, f.into()))
                        .collect(),
                ),
            )
        },
        super::AcquisitionPolicy::ExperimentalTrajectory => {
            let loaded = source
                .load_trajectory_candidates(info.index(), include_weak)?;
            (loaded.candidates, Some(loaded.footprints))
        },
        super::AcquisitionPolicy::SingleInterval => {
            (source.load_rt_candidates(info.index())?, None)
        },
    };
    let native = super::source::validate_peaks(
        NativeFrame {
            peaks: frame.native,
            scan_count: frame.scan_count,
        },
        info,
    )?;
    let weak = super::source::validate_peaks(
        NativeFrame {
            peaks: frame.weak,
            scan_count: frame.scan_count,
        },
        info,
    )?;
    let native_ids: BTreeSet<_> =
        native.iter().map(|p| (p.scan, p.tof)).collect();
    let all: BTreeSet<_> = native
        .iter()
        .chain(&weak)
        .map(|p| (p.scan, p.tof))
        .collect();
    if all.len() != native.len() + weak.len()
        || !frame.eligible.is_subset(&all)
        || weak
            .iter()
            .any(|p| !frame.eligible.contains(&(p.scan, p.tof)))
    {
        return Err(TimsError::new(
            "invalid or duplicate RT candidate provenance",
        ));
    }
    if footprints.as_ref().is_some_and(|f| {
        f.keys().copied().collect::<BTreeSet<_>>() != frame.eligible
            || f.values().any(|p| {
                p.scan_start >= p.scan_end
                    || p.scan_end > frame.scan_count
                    || !p.lower_mz.is_finite()
                    || !p.upper_mz.is_finite()
                    || p.lower_mz <= 0.0
                    || p.lower_mz >= p.upper_mz
                    || (policy
                        == super::AcquisitionPolicy::ExperimentalTrajectory
                        && super::transmission::mass_footprint(
                            info.quadrupole_settings(),
                            p.scan_start,
                            p.scan_end,
                        ) != Some(*p))
            })
    }) {
        return Err(TimsError::new(
            "invalid transmission candidate footprints",
        ));
    }
    let mut peaks = native;
    peaks.extend(weak);
    peaks.sort_by_key(|p| (p.scan, p.tof));
    let responses = peaks
        .iter()
        .map(|p| ((p.scan, p.tof), p.apex_intensity))
        .collect();
    let q = info.quadrupole_settings();
    let mut matching_rows = Vec::with_capacity(q.len());
    for (&start, &end) in q.scan_starts.iter().zip(&q.scan_ends) {
        let lower = peaks.partition_point(|p| (p.scan as usize) < start);
        let upper = peaks.partition_point(|p| (p.scan as usize) < end);
        let mut row = peaks[lower..upper]
            .iter()
            .map(|p| {
                let mz = source.mz(p.tof)?;
                if !mz.is_finite() || mz <= 0.0 {
                    return Err(TimsError::new(
                        "invalid RT matching-domain mass calibration",
                    ));
                }
                Ok(RtSupport {
                    frame: info.index(),
                    rt_s: info.rt_in_seconds(),
                    scan: p.scan,
                    tof: p.tof,
                    mz,
                    response: p.apex_intensity,
                })
            })
            .collect::<TimsResult<Vec<_>>>()?;
        row.sort_by(|a, b| {
            a.mz.total_cmp(&b.mz)
                .then(a.scan.cmp(&b.scan))
                .then(a.tof.cmp(&b.tof))
        });
        matching_rows.push(row);
    }
    Ok((
        peaks,
        FrameMetadata {
            native: native_ids,
            eligible: frame.eligible,
            responses,
            matching_rows,
            footprints,
            policy,
        },
    ))
}

fn close(a: &RtSupport, b: &RtSupport, ppm: f64, radius: usize) -> bool {
    a.scan.abs_diff(b.scan) as usize <= radius
        && (a.mz - b.mz).abs() / ((a.mz + b.mz) * 0.5) * 1e6 <= ppm
}

fn matches(
    a: &RtSupport,
    b: &[RtSupport],
    ppm: f64,
    radius: usize,
) -> Vec<usize> {
    let mz = a.mz;
    let lower = (mz * (2e6 - ppm) / (2e6 + ppm)).next_down();
    let upper = (mz * (2e6 + ppm) / (2e6 - ppm)).next_up();
    let begin = b.partition_point(|p| p.mz < lower);
    (begin..b.len())
        .take_while(|&j| b[j].mz <= upper)
        .filter(|&j| close(a, &b[j], ppm, radius))
        .collect()
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) * 0.5
    } else {
        values[mid]
    }
}

pub(super) fn select(
    neighborhood: &mut Neighborhood,
    config: &RtConsistencyConfig,
    cap: usize,
    scan_radius: usize,
    metadata: &BTreeMap<usize, FrameMetadata>,
    frames: &[FrameInfo],
    reference_mz: f64,
) -> TimsResult<()> {
    let by_id: BTreeMap<_, _> =
        metadata.keys().map(|&i| (frames[i].index(), i)).collect();
    let mut views = vec![(Offset { rt: 0, im: 0 }, &neighborhood.focal)];
    views.extend(neighborhood.context.iter().filter_map(|o| match o {
        Observation::Available { offset, view } => {
            Some((*offset, view.as_ref()))
        },
        Observation::Unavailable { .. } => None,
    }));
    let domains: Vec<_> = views
        .iter()
        .map(|(_, view)| {
            let i = by_id[&view.anchor.frame];
            metadata[&i].domain(view, &frames[i], reference_mz)
        })
        .collect();
    let eligible = |view: &View, i: usize| {
        metadata[&by_id[&view.anchor.frame]]
            .eligible
            .contains(&view.peak_scan_tof[i])
    };
    let mut selected_views = Vec::with_capacity(views.len());
    for (view_index, (offset, view)) in views.iter().enumerate() {
        let domain = &domains[view_index];
        let mut selected = Vec::new();
        for i in 0..view.peak_scan_tof.len() {
            let native = metadata[&by_id[&view.anchor.frame]]
                .native
                .contains(&view.peak_scan_tof[i]);
            let mut members = vec![RtSupport {
                frame: view.anchor.frame,
                rt_s: view.rt_s,
                scan: view.peak_scan_tof[i].0,
                tof: view.peak_scan_tof[i].1,
                mz: f64::from(view.spectrum.mz_values()[i]),
                response: view.peak_apex_responses[i],
            }];
            let mut ambiguous =
                matches(&members[0], domain, config.mz_ppm, scan_radius).len()
                    != 1;
            if eligible(view, i) {
                for (other_index, (other_offset, other)) in
                    views.iter().enumerate()
                {
                    if other_offset.im != offset.im
                        || other.anchor.frame == view.anchor.frame
                    {
                        continue;
                    }
                    let other_metadata = &metadata[&by_id[&other.anchor.frame]];
                    let other_domain = &domains[other_index];
                    let candidates = matches(
                        &members[0],
                        other_domain,
                        config.mz_ppm,
                        scan_radius,
                    );
                    if candidates.len() > 1 {
                        ambiguous = true;
                    } else if let Some(&j) = candidates.first() {
                        let point = &other_domain[j];
                        if matches(point, domain, config.mz_ppm, scan_radius)
                            .len()
                            != 1
                        {
                            ambiguous = true;
                        } else if other_metadata
                            .eligible
                            .contains(&(point.scan, point.tof))
                        {
                            members.push(point.clone());
                        }
                    }
                }
            }
            // A focal-star match must also be a clique: no transitive mass/IM drift.
            for a in 0..members.len() {
                for b in a + 1..members.len() {
                    if !close(
                        &members[a],
                        &members[b],
                        config.mz_ppm,
                        scan_radius,
                    ) {
                        ambiguous = true;
                    }
                }
            }
            let before = members.iter().any(|p| p.rt_s < view.rt_s);
            let after = members.iter().any(|p| p.rt_s > view.rt_s);
            let qualified = eligible(view, i)
                && !ambiguous
                && members.len() >= config.minimum_support
                && (!config.require_both_sides || (before && after));
            if !native && !qualified {
                continue;
            }
            let mut support = members;
            support.sort_by(|a, b| {
                a.rt_s.total_cmp(&b.rt_s).then(a.frame.cmp(&b.frame))
            });
            let score = view.spectrum.intensities()[i]
                * if qualified
                    && config.selection != RtSelectionPolicy::FocalResponse
                {
                    support.len() as f64
                } else {
                    1.0
                };
            let evidence = RtPeakEvidence {
                native,
                qualified,
                ambiguous,
                consensus_mz: median(support.iter().map(|p| p.mz).collect()),
                consensus_scan: median(
                    support.iter().map(|p| p.scan as f64).collect(),
                ),
                support,
                selection_score: score,
            };
            selected.push((i, evidence));
        }
        let native_priority =
            config.selection == RtSelectionPolicy::NativePreserving;
        let native_budget = if native_priority { cap } else { 0 };
        let responses = &metadata[&by_id[&view.anchor.frame]].responses;
        let response_order = |a: usize, b: usize| {
            responses[&view.peak_scan_tof[b]]
                .cmp(&responses[&view.peak_scan_tof[a]])
        };
        let mut protected: Vec<_> = selected
            .iter()
            .filter(|(_, evidence)| evidence.native && native_budget != 0)
            .map(|(i, _)| *i)
            .collect();
        protected.sort_by(|a, b| {
            response_order(*a, *b)
                .then(view.peak_scan_tof[*a].1.cmp(&view.peak_scan_tof[*b].1))
                .then(view.peak_scan_tof[*a].0.cmp(&view.peak_scan_tof[*b].0))
        });
        protected.truncate(native_budget);
        let protected: BTreeSet<_> = protected.into_iter().collect();
        selected.sort_by(|(a, ae), (b, be)| {
            let primary =
                if config.selection == RtSelectionPolicy::FocalResponse {
                    response_order(*a, *b)
                } else {
                    be.selection_score.total_cmp(&ae.selection_score)
                };
            protected
                .contains(b)
                .cmp(&protected.contains(a))
                .then(primary)
                .then_with(|| {
                    if native_priority {
                        response_order(*a, *b)
                    } else {
                        view.spectrum.intensities()[*b]
                            .total_cmp(&view.spectrum.intensities()[*a])
                    }
                })
                .then_with(|| {
                    if config.selection == RtSelectionPolicy::FocalResponse {
                        be.native.cmp(&ae.native)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .then(view.peak_scan_tof[*a].1.cmp(&view.peak_scan_tof[*b].1))
                .then(view.peak_scan_tof[*a].0.cmp(&view.peak_scan_tof[*b].0))
        });
        if cap != 0 {
            selected.truncate(cap);
        }
        selected.sort_by_key(|(i, _)| *i);
        let mut output = (*view).clone();
        output.peak_scan_tof = selected
            .iter()
            .map(|(i, _)| view.peak_scan_tof[*i])
            .collect();
        output.peak_apex_responses = selected
            .iter()
            .map(|(i, _)| view.peak_apex_responses[*i])
            .collect();
        output.peak_inverse_mobility = selected
            .iter()
            .map(|(i, _)| view.peak_inverse_mobility[*i])
            .collect();
        output.spectrum = Spectrum::new(
            selected
                .iter()
                .map(|(i, _)| view.spectrum.intensities()[*i])
                .collect(),
            view.spectrum.index(),
            view.spectrum.precursor().clone(),
            selected
                .iter()
                .map(|(i, _)| view.spectrum.mz_values()[*i])
                .collect(),
            view.spectrum.isolation_window().clone(),
        );
        output.rt_evidence =
            Some(selected.into_iter().map(|(_, evidence)| evidence).collect());
        selected_views.push(output);
    }
    let mut outputs = selected_views.into_iter();
    neighborhood.focal = outputs
        .next()
        .ok_or_else(|| TimsError::new("missing RT focal view"))?;
    for observation in &mut neighborhood.context {
        if let Observation::Available { view, .. } = observation {
            **view = outputs
                .next()
                .ok_or_else(|| TimsError::new("missing RT context view"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
