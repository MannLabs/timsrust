use super::*;
use crate::{Peak, TimsError, TimsResult};
use std::sync::Arc;
use timsrust_core::{
    AcquisitionType, FrameInfo, IsolationWindow, MSLevel, QuadrupoleSettings,
};

struct Source {
    frames: Vec<FrameInfo>,
    fail: bool,
    empty: bool,
}

impl Source {
    fn new() -> Self {
        let q = Arc::new(QuadrupoleSettings {
            index: 1,
            scan_starts: vec![10],
            scan_ends: vec![30],
            isolation_windows: vec![IsolationWindow::new_from_bounds(
                400.0.into(),
                425.0.into(),
                30.0,
            )],
        });
        Self {
            frames: (0..7)
                .map(|i| {
                    FrameInfo::new(
                        q.clone(),
                        i * 10 + 1,
                        i as f64 * 2.0,
                        1.0,
                        AcquisitionType::DIAPASEF,
                        MSLevel::MS2,
                        1,
                        None,
                    )
                })
                .collect(),
            fail: false,
            empty: false,
        }
    }
}

impl NativeSource for Source {
    fn frame_infos(&self) -> TimsResult<Vec<FrameInfo>> {
        Ok(self.frames.clone())
    }
    fn scan_fwhm(&self) -> usize {
        4
    }
    fn mobility(&self, scan: usize) -> TimsResult<f64> {
        Ok(2.0 - scan as f64 / 100.0)
    }
    fn mz(&self, tof: u32) -> TimsResult<f64> {
        Ok(tof as f64 * 2.0)
    }
    fn load_frame(&self, frame: usize) -> TimsResult<NativeFrame> {
        if self.fail {
            return Err(TimsError::new("fixture read failure"));
        }
        Ok(NativeFrame {
            scan_count: 40,
            peaks: if self.empty {
                vec![]
            } else {
                (0..40)
                    .map(|scan| Peak {
                        frame: frame as u32,
                        scan,
                        tof: 100 + scan,
                        apex_intensity: 100 + scan as u64,
                    })
                    .collect()
            },
        })
    }
}

#[test]
fn focal_and_bundle_payloads_match_and_actual_frame_ids_are_preserved() {
    let reader =
        NeighborhoodReader::new(Source::new(), Config::default()).unwrap();
    let spectra: Vec<_> =
        reader.pseudospectra().collect::<TimsResult<_>>().unwrap();
    let bundles: Vec<_> =
        reader.neighborhoods().collect::<TimsResult<_>>().unwrap();
    assert_eq!(spectra.len(), 70);
    for (spectrum, bundle) in spectra.iter().zip(&bundles) {
        assert_eq!(spectrum, &bundle.focal.spectrum);
        assert_eq!(bundle.context.len(), 4);
        assert_eq!(bundle.focal.anchor.frame % 10, 1);
    }
}

#[test]
fn explicit_precursor_coordinate_is_not_snapped_to_the_grid() {
    let reader =
        NeighborhoodReader::new(Source::new(), Config::default()).unwrap();
    let anchor = Anchor {
        frame: 31,
        interval: 0,
        scan: 17,
    };
    let bundle = reader.neighborhood_at(anchor).unwrap();
    assert_eq!(bundle.focal.anchor, anchor);
    assert!(
        bundle
            .context
            .iter()
            .all(|o| matches!(o, Observation::Available { .. }))
    );
    let repeated: Vec<_> = reader
        .neighborhoods_at([anchor, anchor].into_iter())
        .collect::<TimsResult<_>>()
        .unwrap();
    assert_eq!(repeated, vec![bundle.clone(), bundle]);
}

#[test]
fn boundaries_are_masked_and_empty_acquisitions_are_available() {
    let mut source = Source::new();
    source.empty = true;
    let reader = NeighborhoodReader::new(source, Config::default()).unwrap();
    let bundle = reader
        .neighborhood_at(Anchor {
            frame: 1,
            interval: 0,
            scan: 20,
        })
        .unwrap();
    assert!(bundle.focal.spectrum.is_empty());
    assert_eq!(
        bundle
            .context
            .iter()
            .filter(|o| matches!(
                o,
                Observation::Unavailable {
                    reason: UnavailableReason::RunBoundary,
                    ..
                }
            ))
            .count(),
        2
    );
    assert_eq!(
        bundle
            .context
            .iter()
            .filter(|o| matches!(o, Observation::Available { .. }))
            .count(),
        2
    );
}

#[test]
fn caps_are_explicit_and_never_change_surviving_coordinates() {
    let anchor = Anchor {
        frame: 31,
        interval: 0,
        scan: 20,
    };
    let uncapped = NeighborhoodReader::new(
        Source::new(),
        Config {
            peak_cap: 0,
            ..Config::default()
        },
    )
    .unwrap()
    .neighborhood_at(anchor)
    .unwrap();
    let capped = NeighborhoodReader::new(
        Source::new(),
        Config {
            peak_cap: 2,
            ..Config::default()
        },
    )
    .unwrap()
    .neighborhood_at(anchor)
    .unwrap();
    assert_eq!(uncapped.focal.spectrum.len(), 5);
    assert_eq!(capped.focal.spectrum.len(), 2);
    assert!(
        capped
            .focal
            .peak_scan_tof
            .iter()
            .all(|p| uncapped.focal.peak_scan_tof.contains(p))
    );
}

#[test]
fn errors_stop_iteration_and_invalid_configuration_fails() {
    let mut source = Source::new();
    source.fail = true;
    let reader = NeighborhoodReader::new(source, Config::default()).unwrap();
    let mut iter = reader.neighborhoods();
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
    assert!(
        Config {
            im: ImSizing::Scans {
                left: 1,
                right: 1,
                stride: 0
            },
            ..Config::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Config {
            rt_context: ContextExtent {
                before: usize::MAX,
                after: 1
            },
            ..Config::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn strict_context_and_im_cross_products_are_explicit() {
    let config = Config {
        boundary: BoundaryPolicy::RequireComplete,
        im_context: ContextExtent {
            before: 1,
            after: 1,
        },
        ..Config::default()
    };
    let reader = NeighborhoodReader::new(Source::new(), config).unwrap();
    let bundles: Vec<_> =
        reader.neighborhoods().collect::<TimsResult<_>>().unwrap();
    assert!(!bundles.is_empty());
    assert!(
        bundles
            .iter()
            .all(|b| b.context.len() == 14 && b.focal.fully_covered)
    );
    assert!(
        reader
            .neighborhood_at(Anchor {
                frame: 1,
                interval: 0,
                scan: 20
            })
            .is_err()
    );
}
