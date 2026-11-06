use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use timsrust_core::QuadrupoleSettings;

use super::{AcquisitionPolicy, RtCandidateFrame};
use crate::{TimsError, TimsResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransmissionFootprint {
    pub scan_start: usize,
    pub scan_end: usize,
    pub lower_mz: f64,
    pub upper_mz: f64,
    pub collision_energy: f64,
}

impl TransmissionFootprint {
    pub fn contains(&self, mz: f64) -> bool {
        self.lower_mz <= mz && mz < self.upper_mz
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassTransmissionFootprint {
    pub scan_start: usize,
    pub scan_end: usize,
    pub lower_mz: f64,
    pub upper_mz: f64,
}

impl MassTransmissionFootprint {
    pub fn contains(&self, mz: f64) -> bool {
        self.lower_mz <= mz && mz < self.upper_mz
    }
}

impl From<TransmissionFootprint> for MassTransmissionFootprint {
    fn from(f: TransmissionFootprint) -> Self {
        Self {
            scan_start: f.scan_start,
            scan_end: f.scan_end,
            lower_mz: f.lower_mz,
            upper_mz: f.upper_mz,
        }
    }
}

pub struct TransmissionFrame<F = TransmissionFootprint> {
    pub candidates: RtCandidateFrame,
    pub footprints: BTreeMap<(u32, u32), F>,
}

pub(crate) fn footprint(
    q: &QuadrupoleSettings,
    start: usize,
    end: usize,
) -> Option<TransmissionFootprint> {
    let (f, energy) = bounds(q, start, end, true)?;
    Some(TransmissionFootprint {
        scan_start: f.scan_start,
        scan_end: f.scan_end,
        lower_mz: f.lower_mz,
        upper_mz: f.upper_mz,
        collision_energy: energy,
    })
}

pub(crate) fn mass_footprint(
    q: &QuadrupoleSettings,
    start: usize,
    end: usize,
) -> Option<MassTransmissionFootprint> {
    bounds(q, start, end, false).map(|(f, _)| f)
}

/// Intersect every acquisition row, including empty interior raw scans.
fn bounds(
    q: &QuadrupoleSettings,
    start: usize,
    end: usize,
    constant_energy: bool,
) -> Option<(MassTransmissionFootprint, f64)> {
    if start >= end {
        return None;
    }
    let first = q.get_isolation_window_index(start)?;
    let energy = q.isolation_windows[first].collision_energy();
    let mut result = MassTransmissionFootprint {
        scan_start: start,
        scan_end: end,
        lower_mz: 0.0,
        upper_mz: f64::INFINITY,
    };
    let mut scan = start;
    let mut interval = first;
    while scan < end {
        if interval >= q.len()
            || q.scan_starts[interval] > scan
            || q.scan_ends[interval] <= scan
        {
            return None;
        }
        let window = &q.isolation_windows[interval];
        if !window.collision_energy().is_finite()
            || (constant_energy && window.collision_energy() != energy)
        {
            return None;
        }
        result.lower_mz = result.lower_mz.max(f64::from(window.lower()));
        result.upper_mz = result.upper_mz.min(f64::from(window.upper()));
        scan = q.scan_ends[interval];
        interval += 1;
    }
    (result.lower_mz < result.upper_mz).then_some((result, energy))
}

/// Maximal contiguous, constant-energy transmission span for one precursor.
#[cfg(test)]
pub(crate) fn span(
    q: &QuadrupoleSettings,
    interval: usize,
    mz: f64,
) -> Option<(usize, usize)> {
    span_for(q, interval, mz, AcquisitionPolicy::SharedTransmission)
}

pub(crate) fn span_for(
    q: &QuadrupoleSettings,
    interval: usize,
    mz: f64,
    policy: AcquisitionPolicy,
) -> Option<(usize, usize)> {
    let seed = q.isolation_windows.get(interval)?;
    let compatible = |i: usize| {
        let w = &q.isolation_windows[i];
        w.collision_energy().is_finite()
            && (policy == AcquisitionPolicy::ExperimentalTrajectory
                || w.collision_energy() == seed.collision_energy())
            && f64::from(w.lower()) <= mz
            && mz < f64::from(w.upper())
    };
    if !compatible(interval) {
        return None;
    }
    let (mut first, mut last) = (interval, interval);
    while first > 0
        && q.scan_ends[first - 1] == q.scan_starts[first]
        && compatible(first - 1)
    {
        first -= 1;
    }
    while last + 1 < q.len()
        && q.scan_ends[last] == q.scan_starts[last + 1]
        && compatible(last + 1)
    {
        last += 1;
    }
    Some((q.scan_starts[first], q.scan_ends[last]))
}

/// Half-open absolute scan intervals. Masked cells are unobserved, not zeros.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationSegment {
    pub scan_start: i64,
    pub scan_end: i64,
    pub transmitted: bool,
    pub collision_energy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationDomain {
    pub scan_start: i64,
    pub scan_end: i64,
    pub precursor_mz: f64,
    pub segments: Vec<ObservationSegment>,
}

impl ObservationDomain {
    pub fn new(
        q: &QuadrupoleSettings,
        start: i64,
        end: i64,
        mz: f64,
    ) -> TimsResult<Self> {
        if start >= end
            || !mz.is_finite()
            || mz <= 0.0
            || q.len() != q.scan_starts.len()
            || q.len() != q.scan_ends.len()
            || q.scan_starts
                .iter()
                .zip(&q.scan_ends)
                .any(|(&a, &b)| a >= b || i64::try_from(b).is_err())
            || q.scan_ends
                .iter()
                .zip(q.scan_starts.iter().skip(1))
                .any(|(&a, &b)| a > b)
            || q.isolation_windows.iter().any(|w| {
                !w.collision_energy().is_finite()
                    || !f64::from(w.lower()).is_finite()
                    || !f64::from(w.upper()).is_finite()
                    || f64::from(w.lower()) <= 0.0
                    || w.lower() >= w.upper()
            })
        {
            return Err(TimsError::new("invalid observation domain"));
        }
        let mut segments = Vec::new();
        let mut cursor = start;
        for i in 0..q.len() {
            let a = (q.scan_starts[i] as i64).max(start);
            let b = (q.scan_ends[i] as i64).min(end);
            if a >= b {
                continue;
            }
            if cursor < a {
                segments.push(ObservationSegment {
                    scan_start: cursor,
                    scan_end: a,
                    transmitted: false,
                    collision_energy: None,
                });
            }
            let w = &q.isolation_windows[i];
            segments.push(ObservationSegment {
                scan_start: a,
                scan_end: b,
                transmitted: f64::from(w.lower()) <= mz
                    && mz < f64::from(w.upper()),
                collision_energy: Some(w.collision_energy()),
            });
            cursor = b;
        }
        if cursor < end {
            segments.push(ObservationSegment {
                scan_start: cursor,
                scan_end: end,
                transmitted: false,
                collision_energy: None,
            });
        }
        Ok(Self {
            scan_start: start,
            scan_end: end,
            precursor_mz: mz,
            segments,
        })
    }
}

/// Compare masks and observed CE on a common grid, not at shifted peak centers.
pub(crate) fn compatible(
    a: &QuadrupoleSettings,
    b: &QuadrupoleSettings,
    mz: f64,
) -> bool {
    let mut edges: Vec<_> = a
        .scan_starts
        .iter()
        .chain(&a.scan_ends)
        .chain(&b.scan_starts)
        .chain(&b.scan_ends)
        .copied()
        .collect();
    edges.sort_unstable();
    edges.dedup();
    let cell = |q: &QuadrupoleSettings, scan| {
        q.get_isolation_window_index(scan).and_then(|i| {
            let w = &q.isolation_windows[i];
            (f64::from(w.lower()) <= mz && mz < f64::from(w.upper()))
                .then_some(w.collision_energy())
        })
    };
    let mut observed = false;
    for pair in edges.windows(2) {
        let left = cell(a, pair[0]);
        let right = cell(b, pair[0]);
        if left != right || left.is_some_and(|ce| !ce.is_finite()) {
            return false;
        }
        observed |= left.is_some();
    }
    observed
}

#[cfg(test)]
mod tests {
    use super::*;
    use timsrust_core::IsolationWindow;

    #[test]
    fn trajectory_domains_preserve_masks_and_compare_a_common_grid() {
        let a = schedule();
        let mut b = a.clone();
        b.isolation_windows[1] =
            IsolationWindow::new_from_bounds(420.0.into(), 620.0.into(), 31.);
        assert!(footprint(&b, 1, 5).is_none());
        assert!(mass_footprint(&b, 1, 5).unwrap().contains(500.));
        assert_eq!(
            span_for(&b, 1, 500., AcquisitionPolicy::ExperimentalTrajectory),
            Some((0, 6))
        );
        assert!(compatible(&b, &b, 500.));
        assert!(!compatible(&a, &b, 500.));
        let mut changed_mask = b.clone();
        changed_mask.scan_starts[1] = 3;
        assert!(!compatible(&b, &changed_mask, 500.));
        assert!(mass_footprint(&changed_mask, 1, 5).is_none());
        let mut disjoint = b.clone();
        disjoint.isolation_windows[1] =
            IsolationWindow::new_from_bounds(600.0.into(), 700.0.into(), 31.);
        assert!(!compatible(&b, &disjoint, 500.));
        assert!(mass_footprint(&disjoint, 1, 5).is_none());
        let domain = ObservationDomain::new(&disjoint, -1, 7, 600.).unwrap();
        assert_eq!(
            domain
                .segments
                .iter()
                .map(|s| s.transmitted)
                .collect::<Vec<_>>(),
            [false, false, true, false, false]
        );
        assert_eq!(domain.segments[2].collision_energy, Some(31.));
        assert!(ObservationDomain::new(&a, 3, 3, 500.).is_err());
        assert!(ObservationDomain::new(&a, 0, 6, f64::NAN).is_err());
    }

    #[test]
    fn equivalent_row_partitions_match_but_shifted_energy_does_not() {
        let a = schedule();
        let b = QuadrupoleSettings {
            index: 99,
            scan_starts: vec![0],
            scan_ends: vec![6],
            isolation_windows: vec![IsolationWindow::new_from_bounds(
                450.0.into(),
                550.0.into(),
                30.,
            )],
        };
        assert!(compatible(&a, &b, 500.));
        assert!(!compatible(&a, &b, 550.));
        assert!(!compatible(&a, &b, 900.));
        let mut bad = a.clone();
        bad.isolation_windows[0] = IsolationWindow::new_from_bounds(
            400.0.into(),
            600.0.into(),
            f64::NAN,
        );
        assert!(!compatible(&bad, &bad, 500.));
        assert!(ObservationDomain::new(&bad, 0, 6, 500.).is_err());
    }

    #[test]
    fn trajectory_is_explicit_and_unknown_options_are_rejected() {
        let c = super::super::Config {
            acquisition: AcquisitionPolicy::ExperimentalTrajectory,
            ..Default::default()
        };
        c.validate().unwrap();
        let value = serde_json::to_value(&c).unwrap();
        assert_eq!(value["acquisition"], "experimental_trajectory");
        assert_eq!(
            serde_json::from_value::<super::super::Config>(value).unwrap(),
            c
        );
        let mut unknown = serde_json::to_value(&c).unwrap();
        unknown["unknown_option"] = serde_json::json!(true);
        assert!(
            serde_json::from_value::<super::super::Config>(unknown).is_err()
        );
    }

    #[test]
    fn serialization_preserves_defaults_and_rejects_unknown_policies() {
        let value =
            serde_json::to_value(super::super::Config::default()).unwrap();
        assert!(value.get("acquisition").is_none());
        let mut value = value;
        value["acquisition"] = serde_json::json!("shared_transmission");
        let config: super::super::Config =
            serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            config.acquisition,
            super::super::AcquisitionPolicy::SharedTransmission
        );
        value["acquisition"] = serde_json::json!("ignore_acquisition");
        assert!(serde_json::from_value::<super::super::Config>(value).is_err());
    }

    fn schedule() -> QuadrupoleSettings {
        QuadrupoleSettings {
            index: 1,
            scan_starts: vec![0, 2, 4],
            scan_ends: vec![2, 4, 6],
            isolation_windows: [(400., 600.), (420., 620.), (430., 590.)]
                .into_iter()
                .map(|(a, b)| {
                    IsolationWindow::new_from_bounds(a.into(), b.into(), 30.)
                })
                .collect(),
        }
    }

    #[test]
    fn intersection_is_not_union_and_bounds_are_half_open() {
        let q = schedule();
        let f = footprint(&q, 1, 5).unwrap();
        assert_eq!((f.lower_mz, f.upper_mz), (430., 590.));
        assert!(f.contains(430.));
        assert!(!f.contains(590.));
        assert_eq!(span(&q, 1, 500.), Some((0, 6)));
        assert_eq!(span(&q, 0, 600.), None);
        assert_eq!(span(&q, 1, 600.), Some((2, 4)));
    }

    #[test]
    fn gaps_and_energy_changes_are_not_bridged() {
        let mut q = schedule();
        q.scan_starts[1] = 3;
        assert!(footprint(&q, 1, 5).is_none());
        assert_eq!(span(&q, 0, 500.), Some((0, 2)));
        q = schedule();
        q.isolation_windows[2] =
            IsolationWindow::new_from_bounds(430.0.into(), 590.0.into(), 31.);
        assert!(footprint(&q, 1, 5).is_none());
        assert_eq!(span(&q, 0, 500.), Some((0, 4)));
    }

    #[test]
    fn disjoint_windows_and_missing_rows_are_rejected() {
        let mut q = schedule();
        q.isolation_windows[1] =
            IsolationWindow::new_from_bounds(600.0.into(), 800.0.into(), 30.);
        assert!(footprint(&q, 0, 3).is_none());
        assert!(footprint(&q, 0, 7).is_none());
        assert!(footprint(&q, 3, 3).is_none());
    }
}
