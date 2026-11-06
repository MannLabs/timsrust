use crate::{Peak, TimsError, TimsResult, centroider::FrameCentroider};
use std::collections::{BTreeMap, BTreeSet};
use timsrust_core::{Frame, MSLevel, utils::vec::arg_max};

pub(crate) fn validate_frame(frame: &Frame) -> TimsResult<()> {
    let ions = frame.ions();
    let offsets = ions.scan_offsets();
    let q = frame.info().quadrupole_settings();
    if frame.info().ms_level() != MSLevel::MS2
        || u32::try_from(frame.index()).is_err()
        || offsets.len() < 2
        || offsets[0] != 0
        || offsets.last() != Some(&ions.tof_indices().len())
        || ions.tof_indices().len() != ions.intensities().len()
        || offsets.windows(2).any(|p| p[0] > p[1])
        || q.is_empty()
        || q.len() != q.scan_starts.len()
        || q.len() != q.scan_ends.len()
        || q.scan_starts
            .iter()
            .zip(&q.scan_ends)
            .any(|(&a, &b)| a >= b || b >= offsets.len())
        || q.scan_ends
            .iter()
            .zip(q.scan_starts.iter().skip(1))
            .any(|(&a, &b)| a > b)
    {
        return Err(TimsError::new("invalid RT candidate frame layout"));
    }
    for pair in offsets.windows(2) {
        if ions.tof_indices()[pair[0]..pair[1]]
            .windows(2)
            .any(|p| u32::from(p[0]) >= u32::from(p[1]))
        {
            return Err(TimsError::new(
                "RT candidates require strictly increasing raw TOFs per scan",
            ));
        }
    }
    Ok(())
}

pub(crate) fn verified_support(
    frame: &Frame,
    peaks: &mut [Peak],
    centroider: &FrameCentroider,
) -> TimsResult<BTreeSet<(u32, u32)>> {
    let footprints = verified_footprints(frame, peaks, centroider)?;
    let q = frame.info().quadrupole_settings();
    Ok(footprints
        .into_iter()
        .filter_map(|(id, f)| {
            q.get_isolation_window_index(id.0 as usize)
                .filter(|&i| {
                    f.scan_start >= q.scan_starts[i]
                        && f.scan_end <= q.scan_ends[i]
                })
                .map(|_| id)
        })
        .collect())
}

pub(crate) fn verified_footprints(
    frame: &Frame,
    peaks: &mut [Peak],
    centroider: &FrameCentroider,
) -> TimsResult<BTreeMap<(u32, u32), super::TransmissionFootprint>> {
    verified_with(frame, peaks, centroider, super::transmission::footprint)
}

pub(crate) fn verified_mass_footprints(
    frame: &Frame,
    peaks: &mut [Peak],
    centroider: &FrameCentroider,
) -> TimsResult<BTreeMap<(u32, u32), super::MassTransmissionFootprint>> {
    verified_with(
        frame,
        peaks,
        centroider,
        super::transmission::mass_footprint,
    )
}

fn verified_with<F>(
    frame: &Frame,
    peaks: &mut [Peak],
    centroider: &FrameCentroider,
    build: fn(&timsrust_core::QuadrupoleSettings, usize, usize) -> Option<F>,
) -> TimsResult<BTreeMap<(u32, u32), F>> {
    let scan_kernel = centroider.scan_smoother().kernel();
    let scan_apex = arg_max(scan_kernel)
        .ok_or_else(|| TimsError::new("empty scan kernel"))?;
    let tof_kernel = centroider.tof_kernel();
    let tof_apex = arg_max(tof_kernel)
        .ok_or_else(|| TimsError::new("empty TOF kernel"))?;
    let ions = frame.ions();
    let offsets = ions.scan_offsets();
    let scans = ions.scan_count();
    let q = frame.info().quadrupole_settings();
    peaks.sort_by_key(|p| (p.tof, p.scan));
    if peaks.iter().any(|p| {
        p.frame as usize != frame.index()
            || p.scan as usize >= scans
            || p.apex_intensity == 0
    }) || peaks
        .windows(2)
        .any(|p| (p[0].tof, p[0].scan) == (p[1].tof, p[1].scan))
    {
        return Err(TimsError::new("invalid or duplicate RT candidate apex"));
    }
    let mut footprints = BTreeMap::new();
    for p in peaks {
        let upper_tof = i64::from(p.tof) + tof_apex as i64;
        let lower_tof = upper_tof - tof_kernel.len() as i64 + 1;
        let (mut first, mut last, mut response) = (scans, 0, 0u64);
        for (offset, &weight) in scan_kernel.iter().enumerate() {
            let raw_scan = i64::from(p.scan) + scan_apex as i64 - offset as i64;
            if weight == 0 || raw_scan < 0 || raw_scan >= scans as i64 {
                continue;
            }
            let scan = raw_scan as usize;
            let start = offsets[scan];
            let row = &ions.tof_indices()[start..offsets[scan + 1]];
            let begin = row
                .partition_point(|&tof| i64::from(u32::from(tof)) < lower_tof);
            for (index, _) in
                row.iter().enumerate().skip(begin).take_while(|(_, tof)| {
                    i64::from(u32::from(**tof)) <= upper_tof
                })
            {
                let intensity = u64::from(ions.intensities()[start + index]);
                if intensity == 0 {
                    continue;
                }
                response = intensity
                    .checked_mul(weight)
                    .and_then(|v| response.checked_add(v))
                    .ok_or_else(|| {
                        TimsError::new("RT raw response overflow")
                    })?;
                first = first.min(scan);
                last = last.max(scan + 1);
            }
        }
        if first >= last || response != p.apex_intensity {
            return Err(TimsError::new(format!(
                "RT raw response differs at frame {}, scan {}, TOF {}",
                p.frame, p.scan, p.tof
            )));
        }
        if let Some(footprint) =
            build(q, first.min(p.scan as usize), last.max(p.scan as usize + 1))
        {
            footprints.insert((p.scan, p.tof), footprint);
        }
    }
    Ok(footprints)
}
