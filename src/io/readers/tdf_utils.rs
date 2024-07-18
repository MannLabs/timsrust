use crate::io::readers::file_readers::sql_reader::frame_groups::SqlWindowGroup;
use crate::ms_data::QuadrupoleSettings;

type SpanStep = (usize, usize);

/// Strategy for expanding quadrupole settings
///
/// This enum is used to determine how to expand quadrupole settings
/// when reading in DIA data. And exporting spectra (not frames RN).
///
/// # Variants
///
/// For example if we have a window with scan start 50 and end 500
///
/// * `None` - Do not expand quadrupole settings; use the original settings
/// * `Even(usize)` - Split the quadrupole settings into `usize` evenly spaced
/// subwindows; e.g. if `usize` is 2, the window will be split into 2 subwindows
/// of equal width.
/// * `Uniform(SpanStep)` - Split the quadrupole settings into subwindows of
/// width `SpanStep.0` and step `SpanStep.1`; e.g. if `SpanStep` is (100, 50),
/// the window will be split into subwindows of width 100 and step 50 between their
/// scan start and end.
///
#[derive(Debug, Copy, Clone)]
pub enum QuadWindowExpansionStrategy {
    None,
    Even(usize),
    Uniform(SpanStep),
}

fn scan_range_subsplit(
    start: usize,
    end: usize,
    strategy: &QuadWindowExpansionStrategy,
) -> Vec<(usize, usize)> {
    let out = match strategy {
        QuadWindowExpansionStrategy::None => {
            vec![(start, end)]
        },
        QuadWindowExpansionStrategy::Even(num_splits) => {
            let sub_subwindow_width = (end - start) / (num_splits + 1);
            let mut out = Vec::new();
            for sub_subwindow in 0..num_splits.clone() {
                let sub_subwindow_scan_start =
                    start + (sub_subwindow_width * sub_subwindow);
                let sub_subwindow_scan_end =
                    start + (sub_subwindow_width * (sub_subwindow + 2));

                out.push((sub_subwindow_scan_start, sub_subwindow_scan_end))
            }
            out
        },
        QuadWindowExpansionStrategy::Uniform((span, step)) => {
            let mut curr_start = start.clone();
            let mut curr_end = start + span;
            let mut out = Vec::new();
            while curr_end < end {
                out.push((curr_start, curr_end));
                curr_start += step;
                curr_end += step;
            }
            if curr_start < end {
                out.push((curr_start, end));
            }
            out
        },
    };

    debug_assert!(
        out.iter().all(|(s, e)| s < e),
        "Invalid scan range: {:?}",
        out
    );
    debug_assert!(
        out.iter().all(|(s, e)| *s >= start && *e <= end),
        "Invalid scan range: {:?}",
        out
    );
    out
}

pub fn expand_window_settings(
    window_groups: &[SqlWindowGroup],
    quadrupole_settings: &[QuadrupoleSettings],
    strategy: &QuadWindowExpansionStrategy,
) -> Vec<QuadrupoleSettings> {
    let mut expanded_quadrupole_settings: Vec<QuadrupoleSettings> = vec![];
    for window_group in window_groups {
        let window = window_group.window_group;
        let frame = window_group.frame;
        let group = &quadrupole_settings[window as usize - 1];
        let window_group_start =
            group.scan_starts.iter().min().unwrap().clone();
        let window_group_end = group.scan_ends.iter().max().unwrap().clone();

        for (sws, swe) in
            scan_range_subsplit(window_group_start, window_group_end, &strategy)
        {
            let mut mz_sum = 0.0;
            let mut mz_min = std::f64::MAX;
            let mut mz_max = std::f64::MIN;
            let mut nce_sum = 0.0;
            let mut num_added = 0;

            for i in 0..group.isolation_mz.len() {
                // Should I be checking here for overlap instead of full containment?
                if sws <= group.scan_starts[i] && swe >= group.scan_ends[i] {
                    mz_sum += group.isolation_mz[i];
                    mz_min = mz_min.min(
                        group.isolation_mz[i]
                            - (group.isolation_width[i] / 2.0),
                    );
                    mz_max = mz_max.max(
                        group.isolation_mz[i]
                            + (group.isolation_width[i] / 2.0),
                    );
                    nce_sum += group.collision_energy[i];
                    num_added += 1;
                }
            }

            let mz_mean = mz_sum / num_added as f64;
            let mean_nce = nce_sum / num_added as f64;

            let sub_quad_settings = QuadrupoleSettings {
                index: frame,
                scan_starts: vec![sws],
                scan_ends: vec![swe],
                isolation_mz: vec![mz_mean],
                isolation_width: vec![mz_min - mz_max],
                collision_energy: vec![mean_nce],
            };
            expanded_quadrupole_settings.push(sub_quad_settings)
        }
    }
    expanded_quadrupole_settings
}

pub fn expand_quadrupole_settings(
    window_groups: &[SqlWindowGroup],
    quadrupole_settings: &[QuadrupoleSettings],
    strategy: &QuadWindowExpansionStrategy,
) -> Vec<QuadrupoleSettings> {
    // Read the 'NUM_SUB_SUB_SPLITS' from env variables ... default to 1
    // (for now)

    let mut expanded_quadrupole_settings: Vec<QuadrupoleSettings> = vec![];
    for window_group in window_groups {
        let window = window_group.window_group;
        let frame = window_group.frame;
        let group = &quadrupole_settings[window as usize - 1];
        for sub_window in 0..group.isolation_mz.len() {
            let subwindow_scan_start = group.scan_starts[sub_window];
            let subwindow_scan_end = group.scan_ends[sub_window];
            for (sws, swe) in scan_range_subsplit(
                subwindow_scan_start,
                subwindow_scan_end,
                &strategy,
            ) {
                let sub_quad_settings = QuadrupoleSettings {
                    index: frame,
                    scan_starts: vec![sws],
                    scan_ends: vec![swe],
                    isolation_mz: vec![group.isolation_mz[sub_window]],
                    isolation_width: vec![group.isolation_width[sub_window]],
                    collision_energy: vec![group.collision_energy[sub_window]],
                };
                expanded_quadrupole_settings.push(sub_quad_settings)
            }
        }
    }
    expanded_quadrupole_settings
}
