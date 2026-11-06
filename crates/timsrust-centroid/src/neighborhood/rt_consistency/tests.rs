use super::*;
use crate::neighborhood::{
    Anchor, Config, ContextExtent, ImSizing, NeighborhoodReader,
    UnavailableReason, View,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use timsrust_core::{
    AcquisitionType, IsolationWindow, MSLevel, QuadrupoleSettings,
};

struct Source {
    frames: Vec<FrameInfo>,
    native: Vec<Vec<Peak>>,
    weak: Vec<Vec<Peak>>,
    impure: BTreeSet<(u32, u32)>,
    loads: AtomicUsize,
    fail: Option<usize>,
    invalid_mz: Option<u32>,
}

impl Source {
    fn new(count: usize) -> Self {
        let q = Arc::new(QuadrupoleSettings {
            index: 1,
            scan_starts: vec![10],
            scan_ends: vec![30],
            isolation_windows: vec![IsolationWindow::new_from_bounds(
                400.0.into(),
                600.0.into(),
                30.0,
            )],
        });
        let frames: Vec<_> = (0..count)
            .map(|i| {
                FrameInfo::new(
                    q.clone(),
                    100 + 10 * i,
                    2.0 * i as f64,
                    1.0,
                    AcquisitionType::DIAPASEF,
                    MSLevel::MS2,
                    1,
                    None,
                )
            })
            .collect();
        let peak = |i: usize, tof, response| Peak {
            frame: frames[i].index() as u32,
            scan: 20,
            tof,
            apex_intensity: response,
        };
        let native = (0..count)
            .map(|i| {
                let mut peaks = vec![peak(i, 100000, 40 + i as u64)];
                if i == 3 {
                    peaks.push(peak(i, 200000, 100));
                }
                peaks
            })
            .collect();
        let weak = (0..count)
            .map(|i| vec![peak(i, 300000, 30 + i as u64)])
            .collect();
        Self {
            frames,
            native,
            weak,
            impure: BTreeSet::new(),
            loads: AtomicUsize::new(0),
            fail: None,
            invalid_mz: None,
        }
    }
}

impl NativeSource for Source {
    fn load_trajectory_candidates(
        &self,
        frame: usize,
        include_weak: bool,
    ) -> TimsResult<
        crate::neighborhood::TransmissionFrame<
            crate::neighborhood::MassTransmissionFootprint,
        >,
    > {
        let mut candidates = self.load_rt_candidates(frame)?;
        if !include_weak {
            candidates.weak.clear();
            let native: BTreeSet<_> =
                candidates.native.iter().map(|p| (p.scan, p.tof)).collect();
            candidates.eligible.retain(|p| native.contains(p));
        }
        let q = self
            .frames
            .iter()
            .find(|f| f.index() == frame)
            .unwrap()
            .quadrupole_settings();
        let footprints: BTreeMap<_, _> = candidates
            .eligible
            .iter()
            .filter_map(|&id| {
                crate::neighborhood::transmission::mass_footprint(
                    q,
                    id.0 as usize - 1,
                    id.0 as usize + 2,
                )
                .map(|f| (id, f))
            })
            .collect();
        candidates.eligible.retain(|id| footprints.contains_key(id));
        candidates
            .weak
            .retain(|p| candidates.eligible.contains(&(p.scan, p.tof)));
        Ok(crate::neighborhood::TransmissionFrame {
            candidates,
            footprints,
        })
    }

    fn load_transmission_candidates(
        &self,
        frame: usize,
        include_weak: bool,
    ) -> TimsResult<crate::neighborhood::TransmissionFrame> {
        let mut candidates = self.load_rt_candidates(frame)?;
        if !include_weak {
            candidates.weak.clear();
            let native: BTreeSet<_> =
                candidates.native.iter().map(|p| (p.scan, p.tof)).collect();
            candidates.eligible.retain(|p| native.contains(p));
        }
        let footprints = candidates
            .eligible
            .iter()
            .map(|&id| {
                (
                    id,
                    crate::neighborhood::TransmissionFootprint {
                        scan_start: id.0 as usize - 1,
                        scan_end: id.0 as usize + 2,
                        lower_mz: 480.,
                        upper_mz: 520.,
                        collision_energy: 30.,
                    },
                )
            })
            .collect();
        Ok(crate::neighborhood::TransmissionFrame {
            candidates,
            footprints,
        })
    }

    fn frame_infos(&self) -> TimsResult<Vec<FrameInfo>> {
        Ok(self.frames.clone())
    }
    fn scan_fwhm(&self) -> usize {
        4
    }
    fn mz(&self, tof: u32) -> TimsResult<f64> {
        Ok(if self.invalid_mz == Some(tof) {
            f64::NAN
        } else {
            tof as f64 / 200.0
        })
    }
    fn mobility(&self, scan: usize) -> TimsResult<f64> {
        Ok(2.0 - scan as f64 / 100.0)
    }
    fn load_frame(&self, frame: usize) -> TimsResult<NativeFrame> {
        Ok(NativeFrame {
            peaks: self.native[(frame - 100) / 10].clone(),
            scan_count: 40,
        })
    }
    fn load_rt_candidates(&self, frame: usize) -> TimsResult<RtCandidateFrame> {
        self.loads.fetch_add(1, Ordering::Relaxed);
        if self.fail == Some(frame) {
            return Err(TimsError::new("fixture RT I/O failure"));
        }
        let i = (frame - 100) / 10;
        Ok(RtCandidateFrame {
            native: self.native[i].clone(),
            weak: self.weak[i].clone(),
            eligible: self.native[i]
                .iter()
                .chain(&self.weak[i])
                .map(|p| (p.scan, p.tof))
                .filter(|p| !self.impure.contains(p))
                .collect(),
            scan_count: 40,
        })
    }
}

fn config(cap: usize) -> Config {
    Config {
        rt_consistency: Some(RtConsistencyConfig {
            selection: RtSelectionPolicy::SupportWeighted,
            ..RtConsistencyConfig::default()
        }),
        peak_cap: cap,
        ..Config::default()
    }
}

fn anchor(i: usize) -> Anchor {
    Anchor {
        frame: 100 + 10 * i,
        interval: 0,
        scan: 20,
    }
}

fn diagonal_source() -> Source {
    let mut source = Source::new(7);
    let q = Arc::new(QuadrupoleSettings {
        index: 1,
        scan_starts: (0..40).collect(),
        scan_ends: (1..=40).collect(),
        isolation_windows: (0..40)
            .map(|s| {
                IsolationWindow::new_from_bounds(
                    (400. + s as f64 * 0.2).into(),
                    (600. + s as f64 * 0.2).into(),
                    30.,
                )
            })
            .collect(),
    });
    for (i, info) in source.frames.iter_mut().enumerate() {
        *info = FrameInfo::new(
            q.clone(),
            100 + 10 * i,
            2. * i as f64,
            1.,
            AcquisitionType::DIAPASEF,
            MSLevel::MS2,
            1,
            None,
        );
        for peak in &mut source.native[i] {
            peak.scan = 19 + (i % 3) as u32;
        }
    }
    source
}

#[test]
fn shared_transmission_recovers_drifting_diagonal_peaks_without_pooling() {
    use crate::neighborhood::AcquisitionPolicy;
    let request = Config {
        acquisition: AcquisitionPolicy::SharedTransmission,
        peak_cap: 0,
        rt_consistency: Some(RtConsistencyConfig::default()),
        ..Config::default()
    };
    let reader = NeighborhoodReader::new(diagonal_source(), request).unwrap();
    let a = Anchor {
        frame: 130,
        interval: 20,
        scan: 20,
    };
    let n = reader.neighborhood_for_precursor(a, 500.).unwrap();
    assert_eq!((n.focal.scan_start, n.focal.scan_end), (18, 23));
    assert_eq!(n.focal.spectrum.mz_values().len(), 3);
    let evidence = n.focal.rt_evidence.as_ref().unwrap();
    let index = n
        .focal
        .peak_scan_tof
        .iter()
        .position(|p| p.1 == 100000)
        .unwrap();
    assert!(evidence[index].qualified);
    assert_eq!(evidence[index].support.len(), 5);
    assert_eq!(n.focal.peak_apex_responses[index], 43);
    assert_eq!(n.focal.spectrum.intensities()[index], 43.);
    assert_eq!(f64::from(n.focal.spectrum.isolation_window().lower()), 480.);
    assert_eq!(f64::from(n.focal.spectrum.isolation_window().upper()), 520.);
    assert!(
        reader
            .neighborhood_for_precursor(a, 530.)
            .unwrap()
            .focal
            .peak_scan_tof
            .is_empty()
    );
    assert!(reader.neighborhood_for_precursor(a, 900.).is_err());
}

fn trajectory_source(changed: bool) -> Source {
    let mut source = diagonal_source();
    for (i, info) in source.frames.iter_mut().enumerate() {
        let mut q = (**info.quadrupole_settings()).clone();
        q.index = 100 + i;
        for (scan, w) in q.isolation_windows.iter_mut().enumerate() {
            *w = IsolationWindow::new_from_bounds(
                w.lower(),
                w.upper(),
                30. + scan as f64 * 0.1
                    + if changed && i == 2 && scan == 19 {
                        1.
                    } else {
                        0.
                    },
            );
        }
        *info = FrameInfo::new(
            Arc::new(q),
            info.index(),
            info.rt_in_seconds(),
            1.,
            AcquisitionType::DIAPASEF,
            MSLevel::MS2,
            1,
            None,
        );
    }
    source
}

#[test]
fn rejected_weak_candidates_do_not_narrow_the_selected_isolation_summary() {
    use crate::neighborhood::AcquisitionPolicy;
    for cap in [1, 2] {
        let native_config = Config {
            acquisition: AcquisitionPolicy::ExperimentalTrajectory,
            peak_cap: cap,
            ..Config::default()
        };
        let native = NeighborhoodReader::new(
            trajectory_source(false),
            native_config.clone(),
        )
        .unwrap()
        .neighborhood_for_precursor(
            Anchor {
                interval: 20,
                ..anchor(3)
            },
            500.,
        )
        .unwrap();
        let mut source = trajectory_source(false);
        for i in 0..source.frames.len() {
            source.weak[i].push(Peak {
                frame: source.frames[i].index() as u32,
                scan: 22,
                tof: 450000,
                apex_intensity: 10000,
            });
        }
        let admitted = NeighborhoodReader::new(
            source,
            Config {
                rt_consistency: Some(RtConsistencyConfig::default()),
                ..native_config
            },
        )
        .unwrap()
        .neighborhood_for_precursor(
            Anchor {
                interval: 20,
                ..anchor(3)
            },
            500.,
        )
        .unwrap();
        assert_eq!(native.focal.peak_scan_tof, admitted.focal.peak_scan_tof);
        assert_eq!(
            native.focal.spectrum.isolation_window(),
            admitted.focal.spectrum.isolation_window()
        );
    }
}

#[test]
fn trajectory_admission_keeps_native_values_and_rejects_intervening_ce_changes()
{
    use crate::neighborhood::AcquisitionPolicy;
    for cap in [0, 2, 3, 300] {
        let c = Config {
            acquisition: AcquisitionPolicy::ExperimentalTrajectory,
            peak_cap: cap,
            rt_consistency: Some(RtConsistencyConfig {
                selection: RtSelectionPolicy::NativePreserving,
                ..Default::default()
            }),
            ..Default::default()
        };
        let reader =
            NeighborhoodReader::new(trajectory_source(false), c.clone())
                .unwrap();
        let a = Anchor {
            frame: 130,
            interval: 20,
            scan: 20,
        };
        let n = reader.neighborhood_for_precursor(a, 500.).unwrap();
        assert_eq!((n.focal.scan_start, n.focal.scan_end), (18, 23));
        let e = n.focal.rt_evidence.as_ref().unwrap();
        assert_eq!(e.iter().filter(|e| e.native).count(), 2);
        assert_eq!(
            e.iter().filter(|e| !e.native).count(),
            usize::from(cap != 2)
        );
        for (i, &(scan, tof)) in n.focal.peak_scan_tof.iter().enumerate() {
            let response = match tof {
                100000 => 43,
                200000 => 100,
                300000 => 33,
                _ => unreachable!(),
            };
            assert_eq!(n.focal.peak_apex_responses[i], response);
            assert_eq!(
                f64::from(n.focal.spectrum.mz_values()[i]),
                tof as f64 / 200.
            );
            assert_eq!(
                n.focal.peak_inverse_mobility[i],
                2. - scan as f64 / 100.
            );
        }
        let domain = reader.observation_domain(&n.focal).unwrap();
        assert!(domain.segments.iter().all(|s| s.transmitted));
        assert!(
            domain
                .segments
                .windows(2)
                .any(|s| s[0].collision_energy != s[1].collision_energy)
        );
        let changed = NeighborhoodReader::new(trajectory_source(true), c)
            .unwrap()
            .neighborhood_for_precursor(a, 500.)
            .unwrap();
        assert_eq!(
            changed
                .context
                .iter()
                .filter(|o| matches!(
                    o,
                    Observation::Unavailable {
                        reason: UnavailableReason::AcquisitionChanged,
                        ..
                    }
                ))
                .count(),
            2
        );
        assert!(
            changed
                .focal
                .rt_evidence
                .as_ref()
                .unwrap()
                .iter()
                .all(|e| e.native)
        );
    }
}

#[test]
fn trajectory_matches_strict_native_selection_on_constant_ce() {
    use crate::neighborhood::AcquisitionPolicy;
    let make = |acquisition| {
        NeighborhoodReader::new(
            Source::new(7),
            Config {
                acquisition,
                peak_cap: 300,
                rt_consistency: Some(RtConsistencyConfig {
                    selection: RtSelectionPolicy::NativePreserving,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap()
    };
    let a = make(AcquisitionPolicy::SharedTransmission);
    let b = make(AcquisitionPolicy::ExperimentalTrajectory);
    assert_eq!(a.focal.peak_scan_tof, b.focal.peak_scan_tof);
    assert_eq!(a.focal.peak_apex_responses, b.focal.peak_apex_responses);
    assert_eq!(a.focal.spectrum.mz_values(), b.focal.spectrum.mz_values());
    assert_eq!(a.focal.rt_evidence, b.focal.rt_evidence);
}

#[test]
fn shared_native_control_and_shifted_im_context_use_the_same_geometry() {
    use crate::neighborhood::AcquisitionPolicy;
    let reader = NeighborhoodReader::new(
        diagonal_source(),
        Config {
            acquisition: AcquisitionPolicy::SharedTransmission,
            peak_cap: 0,
            im_context: ContextExtent {
                before: 1,
                after: 1,
            },
            ..Config::default()
        },
    )
    .unwrap();
    let n = reader
        .neighborhood_for_precursor(
            Anchor {
                frame: 130,
                interval: 20,
                scan: 20,
            },
            500.,
        )
        .unwrap();
    assert_eq!(n.focal.peak_scan_tof.len(), 2);
    assert!(n.focal.rt_evidence.is_none());
    for o in n.context {
        let Observation::Available { view, .. } = o else {
            panic!("expected covered context")
        };
        assert_eq!(view.anchor.interval, view.anchor.scan);
        assert!(view.fully_covered);
    }
}

#[test]
fn shared_support_precedes_cap_and_keeps_each_measured_value() {
    let baseline = NeighborhoodReader::new(
        Source::new(7),
        Config {
            peak_cap: 2,
            ..Config::default()
        },
    )
    .unwrap()
    .neighborhood_at(anchor(3))
    .unwrap();
    assert_eq!(baseline.focal.peak_scan_tof, [(20, 100000), (20, 200000)]);
    let reader = NeighborhoodReader::new(Source::new(7), config(2)).unwrap();
    let result = reader.neighborhood_at(anchor(3)).unwrap();
    assert_eq!(result.focal.peak_scan_tof, [(20, 100000), (20, 300000)]);
    assert_eq!(result.focal.spectrum.intensities(), &[43.0, 33.0]);
    let evidence = result.focal.rt_evidence.as_ref().unwrap();
    assert!(evidence.iter().all(|e| e.qualified && e.support.len() == 5));
    assert!(evidence[0].native);
    assert!(!evidence[1].native);
    assert_eq!(evidence[1].selection_score, 165.0);
    assert_eq!(
        evidence[1]
            .support
            .iter()
            .map(|p| p.response)
            .collect::<Vec<_>>(),
        [31, 32, 33, 34, 35]
    );
    // The fragment at m/z 1500 is not filtered by precursor isolation 400..600.
    assert_eq!(f64::from(result.focal.spectrum.mz_values()[1]), 1500.0);
}

#[test]
fn focal_response_policy_never_boosts_lower_response_over_a_stronger_focal_peak()
 {
    let mut settings = config(2);
    settings.rt_consistency.as_mut().unwrap().selection =
        RtSelectionPolicy::FocalResponse;
    let result = NeighborhoodReader::new(Source::new(7), settings)
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert_eq!(result.focal.peak_scan_tof, [(20, 100000), (20, 200000)]);
    assert_eq!(result.focal.spectrum.intensities(), &[43.0, 100.0]);
    assert!(result.focal.rt_evidence.unwrap().iter().all(
        |e| e.selection_score
            == e.support.iter().find(|p| p.frame == 130).unwrap().response
                as f64
    ));
}

#[test]
fn focal_response_policy_can_recover_stronger_raw_supported_signal_without_intensity_inversion()
 {
    let mut source = Source::new(7);
    source.weak[3][0].apex_intensity = 101;
    let mut settings = config(2);
    settings.rt_consistency.as_mut().unwrap().selection =
        RtSelectionPolicy::FocalResponse;
    let result = NeighborhoodReader::new(source, settings)
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert_eq!(result.focal.peak_scan_tof, [(20, 200000), (20, 300000)]);
    assert_eq!(result.focal.spectrum.intensities(), &[100.0, 101.0]);
    assert!(!result.focal.rt_evidence.unwrap()[1].native);
}

#[test]
fn focal_response_ties_prefer_native_measurements_even_when_weak_tof_sorts_first()
 {
    let mut source = Source::new(7);
    for peaks in &mut source.weak {
        peaks[0].tof = 50000;
    }
    source.weak[3][0].apex_intensity = 100;
    let mut settings = config(1);
    settings.rt_consistency.as_mut().unwrap().selection =
        RtSelectionPolicy::FocalResponse;
    let result = NeighborhoodReader::new(source, settings)
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert_eq!(result.focal.peak_scan_tof, [(20, 200000)]);
}

#[test]
fn native_only_focal_response_has_exact_baseline_parity_at_every_cap() {
    for cap in [0, 1, 2, 300] {
        let make_source = || {
            let mut source = Source::new(7);
            for peaks in &mut source.weak {
                peaks.clear();
            }
            source
        };
        let baseline = NeighborhoodReader::new(
            make_source(),
            Config {
                peak_cap: cap,
                ..Config::default()
            },
        )
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
        let mut settings = config(cap);
        settings.rt_consistency.as_mut().unwrap().selection =
            RtSelectionPolicy::FocalResponse;
        let result = NeighborhoodReader::new(make_source(), settings)
            .unwrap()
            .neighborhood_at(anchor(3))
            .unwrap();
        assert_eq!(result.focal.spectrum, baseline.focal.spectrum);
        assert_eq!(result.focal.peak_scan_tof, baseline.focal.peak_scan_tof);
    }
}

#[test]
fn native_preserving_fills_spare_capacity_without_displacing_native_peaks() {
    for (cap, expected) in [(0, 3), (1, 1), (2, 2), (3, 3), (300, 3)] {
        let mut source = Source::new(7);
        for peaks in &mut source.weak {
            peaks[0].apex_intensity = 10000;
        }
        let mut settings = config(cap);
        settings.rt_consistency.as_mut().unwrap().selection =
            RtSelectionPolicy::NativePreserving;
        let result = NeighborhoodReader::new(source, settings)
            .unwrap()
            .neighborhood_at(anchor(3))
            .unwrap();
        assert_eq!(result.focal.peak_scan_tof.len(), expected);
        assert!(result.focal.peak_scan_tof.contains(&(20, 200000)));
        if cap != 1 {
            assert!(result.focal.peak_scan_tof.contains(&(20, 100000)));
        }
        assert_eq!(
            result.focal.peak_scan_tof.contains(&(20, 300000)),
            cap == 0 || cap >= 3
        );
    }
}

fn crowded_source() -> Source {
    let mut source = Source::new(7);
    for i in 0..7 {
        source.native[i] = (0..330)
            .map(|j| Peak {
                frame: source.frames[i].index() as u32,
                scan: 20,
                tof: 100000 + j * 1000,
                apex_intensity: 1000 + u64::from(j % 17),
            })
            .collect();
        source.weak[i] = (0..30)
            .map(|j| Peak {
                frame: source.frames[i].index() as u32,
                scan: 20,
                tof: 1000000 + j * 1000,
                apex_intensity: 10000 - u64::from(j),
            })
            .collect();
    }
    source
}

fn available_views(bundle: &Neighborhood) -> impl Iterator<Item = &View> {
    std::iter::once(&bundle.focal).chain(bundle.context.iter().filter_map(
        |o| match o {
            Observation::Available { view, .. } => Some(view.as_ref()),
            Observation::Unavailable { .. } => None,
        },
    ))
}

#[test]
fn protected_policies_match_independent_selection_and_native_payloads_at_every_cap()
 {
    use crate::neighborhood::AcquisitionPolicy;
    for acquisition in [
        AcquisitionPolicy::SingleInterval,
        AcquisitionPolicy::SharedTransmission,
    ] {
        let native = NeighborhoodReader::new(
            crowded_source(),
            Config {
                acquisition,
                peak_cap: 0,
                ..Config::default()
            },
        )
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
        let admitted = NeighborhoodReader::new(
            crowded_source(),
            Config {
                acquisition,
                ..config(0)
            },
        )
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
        for (policy, slots) in [(RtSelectionPolicy::NativePreserving, 0usize)] {
            for cap in [0, 1, 2, 3, 7, 300, 400] {
                let mut settings = config(cap);
                settings.acquisition = acquisition;
                settings.rt_consistency.as_mut().unwrap().selection = policy;
                let result =
                    NeighborhoodReader::new(crowded_source(), settings)
                        .unwrap()
                        .neighborhood_at(anchor(3))
                        .unwrap();
                for ((actual, all), baseline) in available_views(&result)
                    .zip(available_views(&admitted))
                    .zip(available_views(&native))
                {
                    let evidence = all.rt_evidence.as_ref().unwrap();
                    let mut floor: Vec<_> = (0..all.peak_scan_tof.len())
                        .filter(|&i| evidence[i].native)
                        .collect();
                    floor.sort_by_key(|&i| {
                        (
                            std::cmp::Reverse(all.peak_apex_responses[i]),
                            all.peak_scan_tof[i].1,
                            all.peak_scan_tof[i].0,
                        )
                    });
                    if cap != 0 {
                        floor.truncate(cap.saturating_sub(slots));
                    }
                    let floor: BTreeSet<_> = floor.into_iter().collect();
                    let mut rest: Vec<_> = (0..all.peak_scan_tof.len())
                        .filter(|i| !floor.contains(i))
                        .collect();
                    rest.sort_by(|&a, &b| {
                        evidence[b]
                            .selection_score
                            .total_cmp(&evidence[a].selection_score)
                            .then(
                                all.peak_apex_responses[b]
                                    .cmp(&all.peak_apex_responses[a]),
                            )
                            .then(
                                all.peak_scan_tof[a]
                                    .1
                                    .cmp(&all.peak_scan_tof[b].1),
                            )
                            .then(
                                all.peak_scan_tof[a]
                                    .0
                                    .cmp(&all.peak_scan_tof[b].0),
                            )
                    });
                    let mut expected = floor.clone();
                    expected.extend(rest.into_iter().take(if cap == 0 {
                        usize::MAX
                    } else {
                        cap - floor.len()
                    }));
                    assert_eq!(
                        actual.peak_scan_tof,
                        expected
                            .iter()
                            .map(|&i| all.peak_scan_tof[i])
                            .collect::<Vec<_>>(),
                        "{policy:?}, cap {cap}, anchor {:?}",
                        actual.anchor
                    );
                    for (i, id) in actual.peak_scan_tof.iter().enumerate() {
                        let source_index = all
                            .peak_scan_tof
                            .iter()
                            .position(|p| p == id)
                            .unwrap();
                        assert_eq!(
                            actual.peak_apex_responses[i],
                            all.peak_apex_responses[source_index]
                        );
                        assert_eq!(
                            actual.spectrum.intensities()[i],
                            all.spectrum.intensities()[source_index]
                        );
                        assert_eq!(
                            actual.spectrum.mz_values()[i],
                            all.spectrum.mz_values()[source_index]
                        );
                        assert_eq!(
                            actual.peak_inverse_mobility[i],
                            all.peak_inverse_mobility[source_index]
                        );
                        assert_eq!(
                            actual.rt_evidence.as_ref().unwrap()[i],
                            evidence[source_index]
                        );
                    }
                    let mut native_indices: Vec<_> =
                        (0..baseline.peak_scan_tof.len()).collect();
                    native_indices.sort_by_key(|&i| {
                        (
                            std::cmp::Reverse(baseline.peak_apex_responses[i]),
                            baseline.peak_scan_tof[i].1,
                            baseline.peak_scan_tof[i].0,
                        )
                    });
                    if cap != 0 {
                        native_indices.truncate(cap);
                    }
                    let lost = native_indices
                        .iter()
                        .filter(|&&i| {
                            !actual
                                .peak_scan_tof
                                .contains(&baseline.peak_scan_tof[i])
                        })
                        .count();
                    assert!(
                        lost <= slots.min(cap),
                        "{policy:?}, cap {cap}: {lost} losses"
                    );
                    if cap == 300 && actual.anchor == anchor(3) {
                        assert_eq!(lost, slots);
                    }
                }
            }
        }
    }
}

#[test]
fn rt_support_retains_exact_integer_measurements() {
    let response = (1u64 << 53) + 1;
    let mut source = Source::new(7);
    for peaks in &mut source.native {
        peaks[0].apex_intensity = response;
    }
    let result = NeighborhoodReader::new(source, config(1))
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert_eq!(result.focal.peak_apex_responses, [response]);
    let evidence = &result.focal.rt_evidence.as_ref().unwrap()[0];
    assert!(evidence.qualified);
    assert_eq!(evidence.support.len(), 5);
    for support in &evidence.support {
        assert_eq!(
            serde_json::to_value(support).unwrap()["response"].as_u64(),
            Some(response)
        );
    }
}

#[test]
fn protected_native_floor_uses_exact_integer_responses_above_f64_precision() {
    for (policy, cap) in [(RtSelectionPolicy::NativePreserving, 1)] {
        let mut source = Source::new(7);
        for i in 0..7 {
            source.native[i][0].apex_intensity = 1u64 << 53;
            source.weak[i] = [50000, 300000, 400000]
                .map(|tof| Peak {
                    frame: source.frames[i].index() as u32,
                    scan: 20,
                    tof,
                    apex_intensity: 1u64 << 54,
                })
                .to_vec();
        }
        source.native[3][1].apex_intensity = (1u64 << 53) + 1;
        let mut settings = config(cap);
        settings.rt_consistency.as_mut().unwrap().selection = policy;
        let result = NeighborhoodReader::new(source, settings)
            .unwrap()
            .neighborhood_at(anchor(3))
            .unwrap();
        assert!(result.focal.peak_scan_tof.contains(&(20, 200000)));
        assert!(!result.focal.peak_scan_tof.contains(&(20, 100000)));
        let i = result
            .focal
            .peak_scan_tof
            .iter()
            .position(|&id| id == (20, 200000))
            .unwrap();
        assert_eq!(result.focal.peak_apex_responses[i], (1u64 << 53) + 1);
    }
}

#[test]
fn protected_selection_preserves_missing_empty_and_partial_observations() {
    for policy in [RtSelectionPolicy::NativePreserving] {
        for scan in [10, 20] {
            let mut source = Source::new(7);
            source.native[1].clear();
            source.weak[1].clear();
            let mut settings = config(300);
            settings.rt_consistency.as_mut().unwrap().selection = policy;
            let result = NeighborhoodReader::new(source, settings)
                .unwrap()
                .neighborhood_at(Anchor { scan, ..anchor(0) })
                .unwrap();
            assert_eq!(result.focal.fully_covered, scan == 20);
            assert_eq!(
                result
                    .context
                    .iter()
                    .filter(|o| matches!(o, Observation::Unavailable { .. }))
                    .count(),
                2
            );
            let empty = available_views(&result)
                .find(|view| view.anchor.frame == 110)
                .unwrap();
            assert!(empty.peak_scan_tof.is_empty());
            assert!(empty.rt_evidence.as_ref().unwrap().is_empty());
        }
    }
}

#[test]
fn selection_policy_round_trips_and_rejects_unknown_policies() {
    for (name, selection) in [
        ("native_preserving", RtSelectionPolicy::NativePreserving),
        ("support_weighted", RtSelectionPolicy::SupportWeighted),
        ("focal_response", RtSelectionPolicy::FocalResponse),
    ] {
        let parsed: Config = serde_json::from_value(serde_json::json!({
            "rt_consistency": {"selection": name}
        }))
        .unwrap();
        assert_eq!(
            parsed.rt_consistency.as_ref().unwrap().selection,
            selection
        );
        assert_eq!(
            serde_json::to_value(&parsed).unwrap()["rt_consistency"]["selection"],
            name
        );
        assert_eq!(
            parsed,
            serde_json::from_str(&serde_json::to_string(&parsed).unwrap())
                .unwrap()
        );
    }
    let parsed: Config = serde_json::from_str(
        r#"{"rt_consistency":{"selection":"focal_response"}}"#,
    )
    .unwrap();
    assert_eq!(
        parsed.rt_consistency.as_ref().unwrap().selection,
        RtSelectionPolicy::FocalResponse
    );
    assert_eq!(
        parsed,
        serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap()
    );
    assert!(
        serde_json::from_str::<Config>(
            r#"{"rt_consistency":{"selection":"typo"}}"#
        )
        .is_err()
    );
    assert_eq!(
        serde_json::to_value(config(300)).unwrap()["rt_consistency"]["selection"],
        "support_weighted"
    );
    assert_eq!(
        RtConsistencyConfig::default().selection,
        RtSelectionPolicy::NativePreserving
    );
}

#[test]
fn focal_response_ranking_preserves_integer_order_above_f64_precision() {
    let make_source = || {
        let mut source = Source::new(7);
        for peaks in &mut source.weak {
            peaks.clear();
        }
        source.native[3][0].apex_intensity = 1u64 << 53;
        source.native[3][1].apex_intensity = (1u64 << 53) + 1;
        source
    };
    let baseline = NeighborhoodReader::new(
        make_source(),
        Config {
            peak_cap: 1,
            ..Config::default()
        },
    )
    .unwrap()
    .neighborhood_at(anchor(3))
    .unwrap();
    let mut settings = config(1);
    settings.rt_consistency.as_mut().unwrap().selection =
        RtSelectionPolicy::FocalResponse;
    let result = NeighborhoodReader::new(make_source(), settings)
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert_eq!(result.focal.peak_scan_tof, [(20, 200000)]);
    assert_eq!(result.focal.spectrum, baseline.focal.spectrum);
}

#[test]
fn crowded_top300_retains_all_persistent_features_instead_of_one_frame_interference()
 {
    let make_source = || {
        let mut source = Source::new(7);
        for i in 0..7 {
            source.weak[i].clear();
            for j in 0..299 {
                source.native[i].push(Peak {
                    frame: source.frames[i].index() as u32,
                    scan: 20,
                    tof: 400000 + 1000 * j,
                    apex_intensity: 40,
                });
            }
        }
        source
    };
    let anchors = [anchor(2), anchor(3), anchor(4)];
    let native = NeighborhoodReader::new(make_source(), Config::default())
        .unwrap()
        .neighborhoods_at(anchors.into_iter())
        .map(|r| r.unwrap().focal)
        .collect::<Vec<_>>();
    let rt = NeighborhoodReader::new(make_source(), config(300))
        .unwrap()
        .neighborhoods_at(anchors.into_iter())
        .map(|r| r.unwrap().focal)
        .collect::<Vec<_>>();
    let common = |views: &[View]| {
        views
            .iter()
            .map(|v| v.peak_scan_tof.iter().copied().collect::<BTreeSet<_>>())
            .reduce(|a, b| a.intersection(&b).copied().collect())
            .unwrap()
            .len()
    };
    assert_eq!(common(&native), 299);
    assert_eq!(common(&rt), 300);
    assert!(native[1].peak_scan_tof.contains(&(20, 200000)));
    assert!(!rt[1].peak_scan_tof.contains(&(20, 200000)));
    assert!(rt.iter().all(|v| v.spectrum.len() == 300));
}

#[test]
fn im_context_does_not_count_one_acquisition_multiple_times() {
    let mut settings = config(0);
    settings.im_context = ContextExtent {
        before: 1,
        after: 1,
    };
    let result = NeighborhoodReader::new(Source::new(7), settings)
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert!(
        result
            .focal
            .rt_evidence
            .unwrap()
            .iter()
            .all(|e| e.support.len() <= 5)
    );
}

#[test]
fn pseudospectra_really_load_rt_and_batch_caches_are_bounded() {
    let reader = NeighborhoodReader::new(Source::new(7), config(2)).unwrap();
    let results = reader
        .neighborhoods_at([anchor(3), anchor(3), anchor(4)].into_iter())
        .collect::<TimsResult<Vec<_>>>()
        .unwrap();
    assert_eq!(results[0], results[1]);
    assert_eq!(reader.source.loads.load(Ordering::Relaxed), 6);
    reader.source.loads.store(0, Ordering::Relaxed);
    let mut spectra = reader.pseudospectra();
    spectra.next().unwrap().unwrap();
    assert_eq!(reader.source.loads.load(Ordering::Relaxed), 3);
    let mut iter = reader.neighborhoods();
    for result in iter.by_ref() {
        result.unwrap();
    }
    assert!(iter.stats().peak_frames_high_water <= 5);
}

#[test]
fn missing_focal_signal_is_never_copied_from_neighbors() {
    let mut source = Source::new(7);
    source.weak[3].clear();
    let result = NeighborhoodReader::new(source, config(0))
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert!(!result.focal.peak_scan_tof.contains(&(20, 300000)));
}

#[test]
fn boundaries_and_single_frame_do_not_hallucinate_or_panic() {
    for count in [1, 7] {
        let result = NeighborhoodReader::new(Source::new(count), config(0))
            .unwrap()
            .neighborhood_at(anchor(0))
            .unwrap();
        assert_eq!(result.focal.peak_scan_tof, [(20, 100000)]);
        assert!(!result.focal.rt_evidence.unwrap()[0].qualified);
    }
}

#[test]
fn one_to_many_and_many_to_one_ambiguity_veto_weak_recovery() {
    for i in [2, 3] {
        let mut source = Source::new(7);
        let mut extra = source.weak[i][0].clone();
        extra.tof += 1;
        source.weak[i].push(extra);
        let result = NeighborhoodReader::new(source, config(0))
            .unwrap()
            .neighborhood_at(anchor(3))
            .unwrap();
        assert!(!result.focal.peak_scan_tof.contains(&(20, 300000)));
    }
}

#[test]
fn matching_domain_respects_isolation_rows_not_output_crops() {
    for competitor_scan in [19, 21] {
        let mut source = Source::new(7);
        for i in 0..7 {
            let old = &source.frames[i];
            source.frames[i] = FrameInfo::new(
                Arc::new(QuadrupoleSettings {
                    index: 1,
                    scan_starts: vec![10, 20],
                    scan_ends: vec![20, 30],
                    isolation_windows: vec![
                        IsolationWindow::new_from_bounds(
                            400.0.into(),
                            600.0.into(),
                            30.0,
                        ),
                        IsolationWindow::new_from_bounds(
                            600.0.into(),
                            800.0.into(),
                            31.0,
                        ),
                    ],
                }),
                old.index(),
                old.rt_in_seconds(),
                1.0,
                AcquisitionType::DIAPASEF,
                MSLevel::MS2,
                1,
                None,
            );
            source.native[i].clear();
            let mut competitor = source.weak[i][0].clone();
            competitor.scan = competitor_scan;
            competitor.tof += 1;
            source.weak[i].push(competitor);
        }
        let mut settings = config(0);
        settings.im = ImSizing::Scans {
            left: 0,
            right: 0,
            stride: 1,
        };
        let result = NeighborhoodReader::new(source, settings)
            .unwrap()
            .neighborhood_at(Anchor {
                interval: 1,
                ..anchor(3)
            })
            .unwrap();
        assert_eq!(
            result.focal.peak_scan_tof.contains(&(20, 300000)),
            competitor_scan == 19
        );
    }
}

#[test]
fn ineligible_native_competitor_outside_crop_still_vetoes_ambiguity() {
    let mut source = Source::new(7);
    for i in 0..7 {
        source.weak[i][0].scan = 17;
        source.native[i].push(Peak {
            frame: source.frames[i].index() as u32,
            scan: 15,
            tof: 300001,
            apex_intensity: 100,
        });
    }
    source.impure.insert((15, 300001));
    let mut settings = config(0);
    settings.im = ImSizing::Scans {
        left: 4,
        right: 4,
        stride: 1,
    };
    let result = NeighborhoodReader::new(source, settings)
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert!(!result.focal.peak_scan_tof.contains(&(17, 300000)));
}

#[test]
fn shared_evidence_is_identical_across_nested_views_and_shifted_anchors() {
    let mut reference = None;
    for (center, half) in [(20, 4), (20, 10), (21, 4), (21, 10)] {
        let mut source = Source::new(7);
        for i in 0..7 {
            source.weak[i][0].scan = 17;
            source.native[i].push(Peak {
                frame: source.frames[i].index() as u32,
                scan: 11,
                tof: 400000,
                apex_intensity: 70,
            });
        }
        let mut settings = config(0);
        settings.im = ImSizing::Scans {
            left: half,
            right: half,
            stride: 1,
        };
        let result = NeighborhoodReader::new(source, settings)
            .unwrap()
            .neighborhood_at(Anchor {
                scan: center,
                ..anchor(3)
            })
            .unwrap();
        let evidence: BTreeMap<_, _> = result
            .focal
            .peak_scan_tof
            .into_iter()
            .zip(result.focal.rt_evidence.unwrap())
            .filter(|(point, _)| point.0 >= 17)
            .collect();
        assert_eq!(evidence.len(), 3);
        assert_eq!(evidence[&(17, 300000)].support.len(), 5);
        if let Some(expected) = &reference {
            assert_eq!(&evidence, expected);
        } else {
            reference = Some(evidence);
        }
    }
}

#[test]
fn invalid_mass_in_matching_domain_is_not_hidden_by_output_crop() {
    let mut source = Source::new(7);
    source.invalid_mz = Some(400000);
    source.weak[3].push(Peak {
        frame: 130,
        scan: 11,
        tof: 400000,
        apex_intensity: 1,
    });
    let result = NeighborhoodReader::new(source, config(0))
        .unwrap()
        .neighborhood_at(anchor(3));
    assert!(
        result
            .expect_err("invalid off-crop mass calibration was ignored")
            .to_string()
            .contains("mass calibration")
    );
}

#[test]
fn pairwise_guard_rejects_transitive_mass_drift() {
    let mut source = Source::new(7);
    source.weak[1][0].tof += 2;
    source.weak[2][0].tof -= 2;
    let result = NeighborhoodReader::new(source, config(0))
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert!(!result.focal.peak_scan_tof.contains(&(20, 300000)));
}

#[test]
fn changed_acquisition_cannot_be_jumped_over() {
    let mut source = Source::new(7);
    let old = &source.frames[4];
    let mut q = old.quadrupole_settings().as_ref().clone();
    q.isolation_windows[0] =
        IsolationWindow::new_from_bounds(400.0.into(), 600.0.into(), 31.0);
    source.frames[4] = FrameInfo::new(
        Arc::new(q),
        old.index(),
        old.rt_in_seconds(),
        1.0,
        AcquisitionType::DIAPASEF,
        MSLevel::MS2,
        1,
        None,
    );
    let result = NeighborhoodReader::new(source, config(0))
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    for rt in [1, 2] {
        assert!(result.context.iter().any(|o| matches!(o,
            Observation::Unavailable { offset, reason: UnavailableReason::AcquisitionChanged, .. }
            if offset.rt == rt)));
    }
    assert!(!result.focal.peak_scan_tof.contains(&(20, 300000)));
}

#[test]
fn long_acquisition_gap_is_explicit_and_not_a_zero() {
    let mut source = Source::new(7);
    for i in 4..7 {
        let old = &source.frames[i];
        source.frames[i] = FrameInfo::new(
            old.quadrupole_settings().clone(),
            old.index(),
            old.rt_in_seconds() + 100.0,
            1.0,
            AcquisitionType::DIAPASEF,
            MSLevel::MS2,
            1,
            None,
        );
    }
    let result = NeighborhoodReader::new(source, config(0))
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert!(result.context.iter().any(|o| matches!(
        o,
        Observation::Unavailable {
            reason: UnavailableReason::TimeGap,
            ..
        }
    )));
    assert!(!result.focal.peak_scan_tof.contains(&(20, 300000)));
}

#[test]
fn impure_native_support_is_not_promoted_and_bad_provenance_errors() {
    let mut source = Source::new(7);
    source.impure.insert((20, 100000));
    let result = NeighborhoodReader::new(source, config(0))
        .unwrap()
        .neighborhood_at(anchor(3))
        .unwrap();
    assert!(!result.focal.rt_evidence.unwrap()[0].qualified);
    let mut source = Source::new(7);
    source.weak[3].push(source.native[3][0].clone());
    assert!(
        NeighborhoodReader::new(source, config(0))
            .unwrap()
            .neighborhood_at(anchor(3))
            .is_err()
    );
}

#[test]
fn extraction_errors_stop_once_and_configuration_is_strict() {
    let mut source = Source::new(7);
    source.fail = Some(120);
    let reader = NeighborhoodReader::new(source, config(0)).unwrap();
    let mut iter = reader.neighborhoods_at([anchor(3), anchor(4)].into_iter());
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
    for json in [
        r#"{"rt_consistency":{"mz_ppm":0}}"#,
        r#"{"rt_consistency":{"minimum_support":6}}"#,
        r#"{"rt_consistency":{},"rt_context":{"before":0,"after":4}}"#,
        r#"{"rt_consistency":{"mz_pp":10}}"#,
        r#"{"rt_consistency":{},"rt_auto":{}}"#,
        r#"{"rt_consistency":{},"rt_context":{"before":18446744073709551615,"after":1}}"#,
    ] {
        let parsed = serde_json::from_str::<Config>(json);
        assert!(
            parsed.is_err() || parsed.unwrap().validate().is_err(),
            "{json}"
        );
    }
    let setting: Config =
        serde_json::from_str(r#"{"rt_consistency":{}}"#).unwrap();
    assert!(setting.validate().is_ok());
    assert_eq!(
        setting,
        serde_json::from_str(&serde_json::to_string(&setting).unwrap())
            .unwrap()
    );
    assert!(
        Config {
            rt_context: ContextExtent {
                before: 0,
                after: 0
            },
            ..config(0)
        }
        .validate()
        .is_err()
    );
}
