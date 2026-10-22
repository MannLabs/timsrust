use std::collections::BTreeMap;
use std::error::Error;
use std::io;
use std::sync::Arc;

use rayon::prelude::*;
use timsrust_centroid::spectrum_reader::SpectrumReader;
use timsrust_centroid::spectrum_reader::narrow_spectrum_reader::QuadInfo;
use timsrust_centroid::{Peak, PeakReader};
use timsrust_core::utils::reader::{IndexedReader, ParIterableReader, Reader};
use timsrust_core::{
    AcquisitionType, BitConverter, FrameInfo, FrameIons, FrameReader,
    IntensityIndex, IsolationWindow, MSLevel, Mz, QuadrupoleSettings, Spectrum,
    TofIndex,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type RawPoint = (u32, u32, u32);
type MemoryFrameReader =
    FrameReader<MemoryReader<FrameIons>, MemoryReader<FrameInfo>>;
type MemoryPeakReader =
    PeakReader<MemoryReader<FrameIons>, MemoryReader<FrameInfo>>;

struct MemoryReader<T> {
    values: BTreeMap<usize, T>,
}

impl<T: Clone> Reader<T> for MemoryReader<T> {
    type Error = io::Error;

    fn get(&self, index: usize) -> std::result::Result<T, Self::Error> {
        self.values.get(&index).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("synthetic frame {index} does not exist"),
            )
        })
    }
}

impl<T> IndexedReader<T> for MemoryReader<T> {
    type Iter = std::vec::IntoIter<usize>;

    fn iter(&self) -> Self::Iter {
        self.values.keys().copied().collect::<Vec<_>>().into_iter()
    }
}

fn shaped_points(scan: u32, tof: u32) -> Vec<RawPoint> {
    let shape = [1, 2, 4, 2, 1];
    shape
        .iter()
        .enumerate()
        .flat_map(|(s, &scan_weight)| {
            shape.iter().enumerate().map(move |(t, &tof_weight)| {
                (
                    scan + s as u32 - 2,
                    tof + t as u32 - 2,
                    10 * scan_weight * tof_weight,
                )
            })
        })
        .collect()
}

fn ions(points: &[RawPoint]) -> Result<FrameIons> {
    let mut points = points.to_vec();
    points.sort_unstable_by_key(|&(scan, tof, _)| (scan, tof));
    let max_scan = points.iter().map(|p| p.0).max().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "empty synthetic frame")
    })?;
    let mut offsets = vec![0];
    let mut tofs = Vec::new();
    let mut intensities = Vec::new();
    let mut index = 0;
    for scan in 0..=max_scan {
        while index < points.len() && points[index].0 == scan {
            tofs.push(TofIndex::try_from(points[index].1)?);
            intensities.push(IntensityIndex::try_from(points[index].2)?);
            index += 1;
        }
        offsets.push(index);
    }
    Ok(FrameIons::new(offsets, tofs, intensities))
}

fn frame_reader(points: &[RawPoint]) -> Result<MemoryFrameReader> {
    frame_reader_with_ids(points, [1, 2])
}

fn frame_reader_with_ids(
    points: &[RawPoint],
    ids: [usize; 2],
) -> Result<MemoryFrameReader> {
    let settings = Arc::new(QuadrupoleSettings {
        index: 1,
        scan_starts: vec![0],
        scan_ends: vec![200],
        isolation_windows: vec![IsolationWindow::new_from_bounds(
            Mz::from(400.0),
            Mz::from(425.0),
            30.0,
        )],
    });
    let infos = ids
        .into_iter()
        .zip([MSLevel::MS1, MSLevel::MS2])
        .map(|(id, level)| {
            (
                id,
                FrameInfo::new(
                    settings.clone(),
                    id,
                    id as f64 * 0.1,
                    1.0,
                    AcquisitionType::DIAPASEF,
                    level,
                    1,
                    Some(0),
                ),
            )
        })
        .collect();
    let data = BTreeMap::from([
        (ids[0], ions(&shaped_points(100, 100))?),
        (ids[1], ions(points)?),
    ]);
    Ok(FrameReader::new(
        MemoryReader { values: data },
        MemoryReader { values: infos },
    ))
}

fn peak_reader(
    points: &[RawPoint],
    min_count: f64,
) -> Result<MemoryPeakReader> {
    Ok(PeakReader::new(frame_reader(points)?, 1.0, min_count)?)
}

fn peaks(points: &[RawPoint], min_count: f64) -> Result<Vec<Peak>> {
    let mut peaks = peak_reader(points, min_count)?.get_peaks_from_frame(2)?;
    peaks.sort_unstable_by_key(|peak| (peak.scan, peak.tof));
    Ok(peaks)
}

fn print_peaks(name: &str, points: &[RawPoint], min_count: f64) -> Result<()> {
    let peaks = peaks(points, min_count)?;
    let raw_sum: u64 = points.iter().map(|p| u64::from(p.2)).sum();
    println!(
        "{name}: raw_bins={} raw_sum={raw_sum} min_count={min_count} centroids={}",
        points.len(),
        peaks.len()
    );
    if let Some(peak) = peaks.iter().max_by_key(|peak| peak.apex_intensity) {
        println!(
            "  intensity_top1_scan={} intensity_top1_tof={}",
            peak.scan, peak.tof
        );
    }
    for peak in peaks {
        println!(
            "  frame={} scan={} tof={} filtered_apex={}",
            peak.frame, peak.scan, peak.tof, peak.apex_intensity
        );
    }
    Ok(())
}

fn ranking_probe(split: bool) -> Result<(usize, usize, bool)> {
    let mut points: Vec<RawPoint> = (0..299)
        .map(|index| (20, 1000 + index * 8, 2000 + index))
        .collect();
    points.extend([(20, 160, 80), (80, 5000, 1)]);
    if split {
        points.extend([(20, 118, 60), (20, 122, 60)]);
    } else {
        points.push((20, 120, 120));
    }
    let peaks = peaks(&points, 1.0)?;
    let spectrum = Spectrum::new(
        peaks
            .iter()
            .map(|peak| peak.apex_intensity as f64)
            .collect(),
        0,
        None,
        peaks
            .iter()
            .map(|peak| TofIndex::try_from(peak.tof))
            .collect::<std::result::Result<Vec<_>, _>>()?,
        IsolationWindow::new_from_bounds(
            Mz::from(400.0),
            Mz::from(425.0),
            30.0,
        ),
    );
    let selected = spectrum.get_top_n(300);
    let selected_tofs: Vec<u32> = selected
        .tof_indices()
        .iter()
        .map(|tof| u32::from(*tof))
        .collect();
    let target_count = selected_tofs
        .iter()
        .filter(|&&tof| (118..=122).contains(&tof))
        .count();
    let competitor_kept = selected_tofs.contains(&160);
    println!(
        "top300_{}: detected={} selected={} target_centroids_kept={target_count} competitor_kept={competitor_kept}",
        if split { "split" } else { "unsplit" },
        peaks.len(),
        selected.len()
    );
    Ok((selected.len(), target_count, competitor_kept))
}

fn main() -> Result<()> {
    let calibration = peak_reader(&[(20, 120, 100)], 1.0)?;
    println!(
        "synthetic calibration: tof_kernel={} scan_kernel={}",
        calibration.tof_fwhm(),
        calibration.scan_fwhm()
    );
    print_peaks("single_sparse", &[(20, 120, 100)], 1.0)?;
    print_peaks(
        "single_sparse_with_higher_tof_background",
        &[(20, 120, 100), (80, 200, 1)],
        1.0,
    )?;
    print_peaks(
        "equal_sparse_separate_mobility",
        &[(20, 120, 100), (40, 120, 100)],
        1.0,
    )?;
    print_peaks(
        "equal_sparse_separate_mobility_with_background",
        &[(20, 120, 100), (40, 120, 100), (80, 200, 1)],
        1.0,
    )?;
    print_peaks(
        "unequal_sparse_separate_mobility",
        &[(20, 120, 100), (40, 120, 101), (80, 200, 1)],
        1.0,
    )?;
    let mut shaped = shaped_points(20, 120);
    shaped.extend(shaped_points(40, 120));
    print_peaks("equal_shaped_separate_mobility", &shaped, 1.0)?;
    print_peaks(
        "single_sparse_default_ms2_threshold",
        &[(20, 120, 100), (80, 200, 1)],
        2.0,
    )?;
    print_peaks(
        "one_large_bin_default_threshold",
        &[(20, 120, 10000), (80, 200, 1)],
        2.0,
    )?;
    print_peaks(
        "two_small_bins_default_threshold",
        &[(20, 120, 1), (21, 120, 1), (80, 200, 1)],
        2.0,
    )?;
    print_peaks(
        "rank_unsplit",
        &[(20, 120, 120), (20, 160, 80), (80, 200, 1)],
        1.0,
    )?;
    print_peaks(
        "rank_split_same_raw_sum",
        &[(20, 118, 60), (20, 122, 60), (20, 160, 80), (80, 200, 1)],
        1.0,
    )?;
    ranking_probe(false)?;
    ranking_probe(true)?;

    let reader = SpectrumReader::new(
        frame_reader(&shaped_points(20, 120))?,
        1.0,
        1.0,
        1,
        false,
        BitConverter(),
        BitConverter(),
    )?;
    let direct = reader.get_spectra_from_frame(2).len();
    let parallel = reader.par_iter().collect::<Vec<_>>();
    for spectrum in &parallel {
        spectrum.as_ref().map_err(|error| {
            io::Error::other(format!("parallel spectrum read failed: {error}"))
        })?;
    }
    println!(
        "one_based_frame_ids: direct_last_frame_spectra={direct} parallel_spectra={}",
        parallel.len()
    );

    let settings = QuadrupoleSettings {
        index: 1,
        scan_starts: vec![10, 30],
        scan_ends: vec![20, 40],
        isolation_windows: vec![
            IsolationWindow::new_from_bounds(
                Mz::from(400.0),
                Mz::from(425.0),
                30.0,
            ),
            IsolationWindow::new_from_bounds(
                Mz::from(500.0),
                Mz::from(525.0),
                35.0,
            ),
        ],
    };
    let gap_scan = 25;
    let quad = QuadInfo::try_new(&settings, gap_scan);
    println!(
        "isolation_gap: scan={gap_scan} core_window_exists={} spectrum_quad_center={:?}",
        settings.get_isolation_window(gap_scan).is_some(),
        quad.map(|quad| quad.isolation_mz)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_preserves_one_based_ids_and_raw_support() -> Result<()> {
        let reader = frame_reader(&[(20, 120, 100), (40, 120, 101)])?;
        assert_eq!(reader.iter_indices().collect::<Vec<_>>(), vec![1, 2]);
        let frame = reader.get_frame(2)?;
        assert_eq!(frame.ions().scan_count(), 41);
        assert_eq!(frame.ions().read_scan(20).count(), 1);
        assert_eq!(frame.ions().read_scan(30).count(), 0);
        assert_eq!(frame.ions().read_scan(40).count(), 1);
        assert_eq!(
            frame
                .ions()
                .intensities()
                .iter()
                .map(|x| u32::from(*x))
                .sum::<u32>(),
            201
        );
        Ok(())
    }

    #[test]
    fn calibration_and_shaped_peak_control() -> Result<()> {
        let reader = peak_reader(&shaped_points(20, 120), 1.0)?;
        assert_eq!(reader.tof_fwhm(), 3);
        assert_eq!(reader.scan_fwhm(), 3);
        let result = reader.get_peaks_from_frame(2)?;
        assert_eq!(result.len(), 1);
        assert_eq!((result[0].scan, result[0].tof), (20, 120));
        Ok(())
    }

    #[test]
    fn ranking_probe_uses_exactly_300_selected_peaks() -> Result<()> {
        assert_eq!(ranking_probe(false)?, (300, 1, false));
        assert_eq!(ranking_probe(true)?, (300, 0, true));
        Ok(())
    }

    #[test]
    fn disconnected_sparse_peaks_survive_without_a_tof_tail_marker()
    -> Result<()> {
        let points = [(20, 120, 100), (40, 120, 100)];
        let result = peaks(&points, 1.0)?;
        assert_eq!(
            result
                .iter()
                .map(|peak| (peak.scan, peak.tof))
                .collect::<Vec<_>>(),
            vec![(20, 120), (40, 120)]
        );
        let mut with_background = points.to_vec();
        with_background.push((80, 200, 1));
        let mut extended = peaks(&with_background, 1.0)?;
        assert_eq!(extended.len(), 3);
        extended.retain(|peak| peak.tof == 120);
        assert_eq!(result, extended);
        Ok(())
    }

    #[test]
    fn support_threshold_is_unchanged_by_terminal_flushing() -> Result<()> {
        assert!(peaks(&[(20, 120, 10_000)], 2.0)?.is_empty());
        assert_eq!(peaks(&[(20, 120, 1), (21, 120, 1)], 2.0)?.len(), 1);
        Ok(())
    }

    #[test]
    fn wide_iteration_matches_direct_access_for_actual_frame_ids() -> Result<()>
    {
        for ids in [[0, 1], [1, 2], [3, 8]] {
            let reader = SpectrumReader::new(
                frame_reader_with_ids(&shaped_points(20, 120), ids)?,
                1.0,
                1.0,
                1,
                false,
                BitConverter(),
                BitConverter(),
            )?;
            let mut direct = reader.get_spectra_from_frame(ids[1]);
            assert!(!direct.is_empty());
            let mut parallel = reader
                .par_iter()
                .collect::<std::result::Result<Vec<_>, _>>()?;
            direct.sort_unstable_by_key(|spectrum| spectrum.index());
            parallel.sort_unstable_by_key(|spectrum| spectrum.index());
            assert_eq!(direct.len(), parallel.len(), "frame IDs: {ids:?}");
            for (expected, actual) in direct.iter().zip(&parallel) {
                assert_eq!(actual.index(), expected.index());
                assert_eq!(actual.index() >> 32, ids[1]);
                assert_eq!(actual.tof_indices(), expected.tof_indices());
                assert_eq!(actual.intensities(), expected.intensities());
            }
        }
        Ok(())
    }
}
