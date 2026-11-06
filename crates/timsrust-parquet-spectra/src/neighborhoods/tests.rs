use super::*;
use std::sync::Arc;
use timsrust_centroid::{
    Peak, TimsResult,
    neighborhood::{
        AcquisitionPolicy, MassTransmissionFootprint, NativeFrame,
        NativeSource, NeighborhoodReader, RtCandidateFrame,
        RtConsistencyConfig, TransmissionFootprint, TransmissionFrame,
    },
};
use timsrust_core::{AcquisitionType, MSLevel, QuadrupoleSettings};

struct Source {
    empty: bool,
}
impl NativeSource for Source {
    fn scan_fwhm(&self) -> usize {
        4
    }
    fn frame_infos(&self) -> TimsResult<Vec<FrameInfo>> {
        let q = Arc::new(QuadrupoleSettings {
            index: 1,
            scan_starts: vec![0],
            scan_ends: vec![40],
            isolation_windows: vec![IsolationWindow::new_from_bounds(
                400.0.into(),
                600.0.into(),
                30.0,
            )],
        });
        Ok((0..7)
            .map(|i| {
                FrameInfo::new(
                    q.clone(),
                    10 * i + 1,
                    2.0 * i as f64,
                    1.0,
                    AcquisitionType::DIAPASEF,
                    MSLevel::MS2,
                    1,
                    None,
                )
            })
            .collect())
    }
    fn load_frame(&self, frame: usize) -> TimsResult<NativeFrame> {
        Ok(NativeFrame {
            scan_count: 40,
            peaks: if self.empty {
                vec![]
            } else {
                vec![Peak {
                    frame: frame as u32,
                    scan: 20,
                    tof: 100,
                    apex_intensity: (1u64 << 53) + 1,
                }]
            },
        })
    }
    fn load_rt_candidates(&self, frame: usize) -> TimsResult<RtCandidateFrame> {
        let native = self.load_frame(frame)?.peaks;
        let weak = vec![Peak {
            frame: frame as u32,
            scan: 20,
            tof: 500,
            apex_intensity: 7,
        }];
        Ok(RtCandidateFrame {
            eligible: native
                .iter()
                .chain(&weak)
                .map(|p| (p.scan, p.tof))
                .collect(),
            native,
            weak,
            scan_count: 40,
        })
    }
    fn load_transmission_candidates(
        &self,
        frame: usize,
        include_weak: bool,
    ) -> TimsResult<TransmissionFrame> {
        let mut candidates = self.load_rt_candidates(frame)?;
        if !include_weak {
            candidates.weak.clear();
            candidates.eligible =
                candidates.native.iter().map(|p| (p.scan, p.tof)).collect();
        }
        let footprints = candidates
            .native
            .iter()
            .chain(&candidates.weak)
            .map(|p| {
                (
                    (p.scan, p.tof),
                    TransmissionFootprint {
                        scan_start: 18,
                        scan_end: 23,
                        lower_mz: 400.0,
                        upper_mz: 600.0,
                        collision_energy: 30.0,
                    },
                )
            })
            .collect();
        Ok(TransmissionFrame {
            candidates,
            footprints,
        })
    }
    fn load_trajectory_candidates(
        &self,
        frame: usize,
        include_weak: bool,
    ) -> TimsResult<TransmissionFrame<MassTransmissionFootprint>> {
        let frame = self.load_transmission_candidates(frame, include_weak)?;
        Ok(TransmissionFrame {
            candidates: frame.candidates,
            footprints: frame
                .footprints
                .into_iter()
                .map(|(id, footprint)| (id, footprint.into()))
                .collect(),
        })
    }
    fn mz(&self, tof: u32) -> TimsResult<f64> {
        Ok(50.0 + tof as f64)
    }
    fn mobility(&self, scan: usize) -> TimsResult<f64> {
        Ok(2.0 - scan as f64 / 100.0)
    }
}

#[test]
fn all_acquisition_and_rt_modes_round_trip_values_masks_and_provenance() {
    for (acquisition, rt) in [
        AcquisitionPolicy::SingleInterval,
        AcquisitionPolicy::SharedTransmission,
        AcquisitionPolicy::ExperimentalTrajectory,
    ]
    .into_iter()
    .flat_map(|policy| [false, true].map(|rt| (policy, rt)))
    {
        let config = Config {
            acquisition,
            peak_cap: 0,
            rt_consistency: rt.then(RtConsistencyConfig::default),
            ..Config::default()
        };
        let native =
            NeighborhoodReader::new(Source { empty: false }, config).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("observations");
        let mut writer = Writer::new(
            &path,
            native.resolved_config(),
            "window",
            native.frame_infos(),
        )
        .unwrap();
        let bundles: Vec<_> = [1, 31]
            .into_iter()
            .map(|frame| {
                native
                    .neighborhood_at(Anchor {
                        frame,
                        interval: 0,
                        scan: 20,
                    })
                    .unwrap()
            })
            .collect();
        for bundle in &bundles {
            writer.write(bundle, None).unwrap();
        }
        let manifest = writer.finish().unwrap();
        assert_eq!(manifest.bundles, 2);
        let reader = Reader::new(&path).unwrap();
        assert_eq!(reader.acquisition().unwrap().len(), 7);
        for (i, bundle) in bundles.iter().enumerate() {
            assert_eq!(&reader.get(i).unwrap().neighborhood, bundle);
        }
        assert_eq!(
            bundles[1].focal.peak_scan_tof.len(),
            if rt { 2 } else { 1 }
        );
        assert_eq!(bundles[1].focal.peak_apex_responses[0], (1u64 << 53) + 1);
        assert!(reader.get(2).is_err());
    }
}

#[test]
fn empty_measured_observations_and_zero_bundle_exports_are_valid() {
    let native =
        NeighborhoodReader::new(Source { empty: true }, Config::default())
            .unwrap();
    for count in [0, 1] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dataset");
        let mut writer = Writer::new(
            &path,
            native.resolved_config(),
            "explicit",
            native.frame_infos(),
        )
        .unwrap();
        let bundle = native
            .neighborhood_at(Anchor {
                frame: 31,
                interval: 0,
                scan: 20,
            })
            .unwrap();
        if count == 1 {
            writer.write(&bundle, None).unwrap();
        }
        writer.finish().unwrap();
        let reader = Reader::new(&path).unwrap();
        assert_eq!(reader.manifest.bundles, count);
        if count == 1 {
            assert_eq!(reader.get(0).unwrap().neighborhood, bundle);
        }
    }
}

#[test]
fn incomplete_unknown_schema_and_overwrite_are_rejected() {
    let native =
        NeighborhoodReader::new(Source { empty: false }, Config::default())
            .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dataset");
    let writer = Writer::new(
        &path,
        native.resolved_config(),
        "window",
        native.frame_infos(),
    )
    .unwrap();
    assert!(Reader::new(&path).is_err());
    assert!(
        Writer::new(
            &path,
            native.resolved_config(),
            "window",
            native.frame_infos()
        )
        .is_err()
    );
    writer.finish().unwrap();
    let mut manifest: Manifest =
        serde_json::from_slice(&fs::read(path.join("manifest.json")).unwrap())
            .unwrap();
    manifest.schema_version = 999;
    fs::write(
        path.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(Reader::new(&path).is_err());
}

#[test]
fn failed_writes_cannot_publish_a_completion_manifest() {
    let native =
        NeighborhoodReader::new(Source { empty: false }, Config::default())
            .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dataset");
    let mut writer = Writer::new(
        &path,
        native.resolved_config(),
        "precursor",
        native.frame_infos(),
    )
    .unwrap();
    let bundle = native
        .neighborhood_at(Anchor {
            frame: 31,
            interval: 0,
            scan: 20,
        })
        .unwrap();
    assert!(writer.write(&bundle, None).is_err());
    assert!(writer.finish().is_err());
    assert!(!path.join("manifest.json").exists());
}

#[test]
fn measured_precursor_and_deliberate_prefix_round_trip() {
    let native =
        NeighborhoodReader::new(Source { empty: false }, Config::default())
            .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dataset");
    let mut writer = Writer::new(
        &path,
        native.resolved_config(),
        "precursor",
        native.frame_infos(),
    )
    .unwrap();
    assert!(writer.set_bundle_limit(0).is_err());
    writer.set_bundle_limit(1).unwrap();
    let bundle = native
        .neighborhood_at(Anchor {
            frame: 31,
            interval: 0,
            scan: 20,
        })
        .unwrap();
    let precursor = Precursor::new(
        500.0.into(),
        1.8.into(),
        5.5.into(),
        20u32.try_into().unwrap(),
        Some(2i8.try_into().unwrap()),
        Some(123.0),
        17,
        21u32.try_into().unwrap(),
    );
    writer.write(&bundle, Some(&precursor)).unwrap();
    assert!(writer.set_bundle_limit(2).is_err());
    assert_eq!(writer.finish().unwrap().bundle_limit, Some(1));
    let restored = Reader::new(&path).unwrap().get(0).unwrap();
    assert_eq!(restored.neighborhood, bundle);
    assert_eq!(
        precursor_json(restored.measured_precursor.as_ref().unwrap()).unwrap(),
        precursor_json(&precursor).unwrap()
    );
}

#[test]
fn exceeding_deliberate_prefix_cannot_publish_completion() {
    let native =
        NeighborhoodReader::new(Source { empty: false }, Config::default())
            .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dataset");
    let mut writer = Writer::new(
        &path,
        native.resolved_config(),
        "window",
        native.frame_infos(),
    )
    .unwrap();
    writer.set_bundle_limit(1).unwrap();
    let bundle = native
        .neighborhood_at(Anchor {
            frame: 31,
            interval: 0,
            scan: 20,
        })
        .unwrap();
    writer.write(&bundle, None).unwrap();
    assert!(writer.write(&bundle, None).is_err());
    assert!(writer.finish().is_err());
    assert!(!path.join("manifest.json").exists());
}

#[test]
fn metadata_stream_preserves_metadata_and_stops_on_input_error() {
    let native =
        NeighborhoodReader::new(Source { empty: false }, Config::default())
            .unwrap();
    let anchor = Anchor {
        frame: 31,
        interval: 0,
        scan: 20,
    };
    let inputs = vec![
        Ok((anchor, 17)),
        Err(timsrust_centroid::TimsError::new("source failed")),
        Ok((anchor, 19)),
    ];
    let mut stream = native.neighborhoods_with(inputs.into_iter());
    let (bundle, metadata) = stream.next().unwrap().unwrap();
    assert_eq!(metadata, 17);
    assert_eq!(bundle.focal.anchor, anchor);
    assert!(stream.next().unwrap().is_err());
    assert!(stream.next().is_none());
}
