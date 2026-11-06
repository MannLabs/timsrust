use timsrust::{
    NativeNeighborhoodReader,
    centroid::{TimsError, TimsResult, neighborhood::Config},
};
use timsrust_parquet_spectra::neighborhoods::{Manifest, Writer};

/// Exports a fresh versioned dataset; both sampling policies support RT on/off.
pub fn export_neighborhoods(
    input: &str,
    output: &str,
    config: Config,
    use_precursors: bool,
    min_ms1_ion_count: f64,
    max_bundles: Option<usize>,
) -> TimsResult<Manifest> {
    let reader = NativeNeighborhoodReader::new(input, config)
        .map_err(|e| TimsError::new(e.to_string()))?;
    let mut writer = Writer::new(
        output,
        reader.resolved_config(),
        if use_precursors {
            "precursor"
        } else {
            "window"
        },
        reader.frame_infos(),
    )
    .map_err(|e| TimsError::new(e.to_string()))?;
    if let Some(limit) = max_bundles {
        writer
            .set_bundle_limit(limit)
            .map_err(|e| TimsError::new(e.to_string()))?;
    }
    if use_precursors {
        for item in reader
            .precursor_neighborhoods(min_ms1_ion_count)
            .map_err(|e| TimsError::new(e.to_string()))?
            .take(max_bundles.unwrap_or(usize::MAX))
        {
            let (bundle, precursor) = item?;
            writer
                .write(&bundle, Some(&precursor))
                .map_err(|e| TimsError::new(e.to_string()))?;
        }
    } else {
        for item in reader
            .neighborhoods()
            .take(max_bundles.unwrap_or(usize::MAX))
        {
            writer
                .write(&item?, None)
                .map_err(|e| TimsError::new(e.to_string()))?;
        }
    }
    writer.finish().map_err(|e| TimsError::new(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use timsrust::centroid::neighborhood::{
        AcquisitionPolicy, RtConsistencyConfig,
    };
    use timsrust_parquet_spectra::neighborhoods::Reader;

    #[test]
    #[ignore = "set TIMSRUST_TEST_DIA to a raw DIA dataset with detected MS1 precursors"]
    fn raw_exports_round_trip_with_fixed_anchor_populations() {
        let input = std::env::var("TIMSRUST_TEST_DIA")
            .expect("TIMSRUST_TEST_DIA is required");
        let dir = tempfile::tempdir().unwrap();
        for (acquisition, peak_cap, precursor) in [
            AcquisitionPolicy::SingleInterval,
            AcquisitionPolicy::SharedTransmission,
            AcquisitionPolicy::ExperimentalTrajectory,
        ]
        .into_iter()
        .flat_map(|policy| {
            [0, 300].into_iter().flat_map(move |cap| {
                [false, true].map(|precursor| (policy, cap, precursor))
            })
        }) {
            let mut native_anchors = None;
            for rt in [false, true] {
                let path = dir.path().join(format!(
                    "{acquisition:?}-{peak_cap}-{precursor}-{rt}"
                ));
                let manifest = export_neighborhoods(
                    &input,
                    path.to_str().unwrap(),
                    Config {
                        acquisition,
                        peak_cap,
                        rt_consistency: rt.then(RtConsistencyConfig::default),
                        ..Config::default()
                    },
                    precursor,
                    0.5,
                    Some(5),
                )
                .unwrap();
                assert_eq!(manifest.bundles, 5);
                assert_eq!(manifest.bundle_limit, Some(5));
                let reader = Reader::new(&path).unwrap();
                let bundles: Vec<_> =
                    (0..5).map(|i| reader.get(i).unwrap()).collect();
                assert!(
                    bundles.iter().any(|b| !b
                        .neighborhood
                        .focal
                        .peak_scan_tof
                        .is_empty())
                );
                assert!(
                    bundles
                        .iter()
                        .all(|b| b.measured_precursor.is_some() == precursor)
                );
                let anchors: Vec<_> = bundles
                    .iter()
                    .map(|b| b.neighborhood.focal.anchor)
                    .collect();
                if let Some(expected) = &native_anchors {
                    assert_eq!(&anchors, expected);
                } else {
                    native_anchors = Some(anchors);
                }
            }
        }
    }
}
