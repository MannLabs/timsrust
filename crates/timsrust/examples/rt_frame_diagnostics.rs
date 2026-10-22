use std::error::Error;
use std::io;
use std::time::Instant;

use timsrust_centroid::{Peak, PeakReader};
use timsrust_core::{FrameInfo, MSLevel};
use timsrust_tdf::TdfFrameReader;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn match_count(
    peaks: &[Peak],
    reference: &[Peak],
    scan_tol: u32,
    tof_tol: u32,
) -> usize {
    peaks
        .iter()
        .filter(|peak| {
            let left = reference.partition_point(|other| {
                other.tof < peak.tof.saturating_sub(tof_tol)
            });
            reference[left..]
                .iter()
                .take_while(|other| {
                    other.tof <= peak.tof.saturating_add(tof_tol)
                })
                .any(|other| other.scan.abs_diff(peak.scan) <= scan_tol)
        })
        .count()
}

fn select_peaks(
    peaks: &[Peak],
    scan: u32,
    half_width: u32,
    budget: usize,
) -> Vec<Peak> {
    let mut selected: Vec<Peak> = peaks
        .iter()
        .filter(|peak| peak.scan.abs_diff(scan) <= half_width)
        .cloned()
        .collect();
    selected.sort_unstable_by(|a, b| {
        b.apex_intensity
            .cmp(&a.apex_intensity)
            .then_with(|| a.tof.cmp(&b.tof))
            .then_with(|| a.scan.cmp(&b.scan))
    });
    selected.truncate(budget);
    selected.sort_unstable_by_key(|peak| peak.tof);
    selected
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: rt_frame_diagnostics <run.d> <center_rt_seconds> <window_group>",
        )
        .into());
    }
    let path = &args[0];
    let center: f64 = args[1].parse()?;
    let group: u8 = args[2].parse()?;
    if !center.is_finite() || center < 0.0 || group == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "center RT must be finite and nonnegative; window group must be positive",
        )
        .into());
    }

    let frame_reader = TdfFrameReader::new(path.as_str())?.into_inner();
    let mut infos: Vec<FrameInfo> = frame_reader
        .iter_indices()
        .map(|id| frame_reader.get_info(id))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    infos.retain(|info| {
        info.ms_level() == MSLevel::MS2 && info.window_group() == group
    });
    infos.sort_unstable_by(|a, b| {
        a.rt_in_seconds().total_cmp(&b.rt_in_seconds())
    });
    let center_index = infos
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (a.rt_in_seconds() - center)
                .abs()
                .total_cmp(&(b.rt_in_seconds() - center).abs())
        })
        .map(|(index, _)| index)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no MS2 frames for window group",
            )
        })?;
    if center_index < 2 || center_index + 2 >= infos.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "chosen center does not have two preceding and two following compatible observations",
        )
        .into());
    }
    let infos = &infos[center_index - 2..=center_index + 2];
    if infos.iter().any(|info| {
        info.quadrupole_settings() != infos[2].quadrupole_settings()
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "window group does not have identical quadrupole settings across the selected observations",
        )
        .into());
    }
    let started = Instant::now();
    let reader = PeakReader::new(frame_reader, 0.5, 2.0)?;
    let scan_tol = u32::try_from(reader.scan_fwhm() / 2)?;
    let tof_tol = u32::try_from(reader.tof_fwhm() / 2)?;
    println!("# run={path}");
    println!(
        "# tof_kernel={} scan_kernel={} min_count_ms1={} min_count_ms2={} kernel_estimation_seconds={:.3}",
        reader.tof_fwhm(),
        reader.scan_fwhm(),
        reader.min_count_ms1(),
        reader.min_count_ms2(),
        started.elapsed().as_secs_f64()
    );
    let peak_sets: Vec<Vec<Peak>> = infos
        .iter()
        .map(|info| reader.get_peaks_from_frame(info.index()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let anchor = peak_sets[2]
        .iter()
        .filter(|peak| {
            infos[2]
                .quadrupole_settings()
                .get_isolation_window(peak.scan as usize)
                .is_some()
        })
        .max_by_key(|peak| peak.apex_intensity)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "central frame has no in-window centroids",
            )
        })?
        .scan;
    println!(
        "# fixed_scan_center={anchor} scan_half_width={scan_tol} match_tof_tolerance={tof_tol} match_scan_tolerance={scan_tol}"
    );
    println!(
        "# exploratory fixed-IM slice; intensity-only top-N; no deisotoping, calibrated m/z, peptide labels, or precursor gating"
    );
    println!(
        "frame,rt_s,raw_bins,centroids,outside_quadrupole,no_local_raw_support,slice_peaks,top150,top300,top512,slice_matches_previous,top300_matches_previous"
    );
    let mut previous_slice = Vec::new();
    let mut previous_top300 = Vec::new();
    for (observation_index, (info, peaks)) in
        infos.iter().zip(&peak_sets).enumerate()
    {
        let frame = reader.frame_reader().get_frame(info.index())?;
        let outside = peaks
            .iter()
            .filter(|peak| {
                info.quadrupole_settings()
                    .get_isolation_window(peak.scan as usize)
                    .is_none()
            })
            .count();
        let unsupported = peaks
            .iter()
            .filter(|peak| {
                !(peak.scan.saturating_sub(scan_tol)
                    ..=peak.scan.saturating_add(scan_tol))
                    .any(|scan| {
                        frame.ions().read_scan(scan as usize).any(|(tof, _)| {
                            u32::from(tof).abs_diff(peak.tof) <= tof_tol
                        })
                    })
            })
            .count();
        let slice = select_peaks(peaks, anchor, scan_tol, usize::MAX);
        let top150 = select_peaks(peaks, anchor, scan_tol, 150);
        let top300 = select_peaks(peaks, anchor, scan_tol, 300);
        let top512 = select_peaks(peaks, anchor, scan_tol, 512);
        let (before, after) = if observation_index == 0 {
            ("NA".to_string(), "NA".to_string())
        } else {
            (
                match_count(&slice, &previous_slice, scan_tol, tof_tol)
                    .to_string(),
                match_count(&top300, &previous_top300, scan_tol, tof_tol)
                    .to_string(),
            )
        };
        println!(
            "{},{:.6},{},{},{},{},{},{},{},{},{},{}",
            info.index(),
            info.rt_in_seconds(),
            frame.len(),
            peaks.len(),
            outside,
            unsupported,
            slice.len(),
            top150.len(),
            top300.len(),
            top512.len(),
            before,
            after
        );
        previous_slice = slice;
        previous_top300 = top300;
    }
    println!("# elapsed_seconds={:.3}", started.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(scan: u32, tof: u32, intensity: u64) -> Peak {
        Peak {
            frame: 1,
            scan,
            tof,
            apex_intensity: intensity,
        }
    }

    #[test]
    fn selection_respects_slice_budget_and_ties() {
        let peaks =
            vec![peak(10, 100, 5), peak(10, 110, 5), peak(20, 120, 100)];
        assert_eq!(select_peaks(&peaks, 10, 1, 1), vec![peak(10, 100, 5)]);
        assert!(select_peaks(&peaks, 30, 1, 300).is_empty());
    }

    #[test]
    fn matching_is_inclusive_and_requires_both_dimensions() {
        let reference = vec![peak(10, 100, 5), peak(20, 110, 5)];
        let peaks = vec![peak(12, 102, 1), peak(30, 110, 1), peak(10, 90, 1)];
        assert_eq!(match_count(&peaks, &reference, 2, 2), 1);
    }
}
