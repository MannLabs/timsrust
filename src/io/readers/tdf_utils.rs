use crate::io::readers::file_readers::sql_reader::frame_groups::SqlWindowGroup;
use crate::ms_data::QuadrupoleSettings;

type SpanStep = (usize, usize);

enum QuadWindowExpansionStrategy {
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
            out
        },
    };
    out
}

pub fn expand_quadrupole_settings(
    window_groups: &[SqlWindowGroup],
    quadrupole_settings: &[QuadrupoleSettings],
) -> Vec<QuadrupoleSettings> {
    // Read the 'NUM_SUB_SUB_SPLITS' from env variables ... default to 1
    // (for now)

    let splits = match std::env::var("NUM_SUB_SUB_SPLITS") {
        Ok(s) => match s.parse::<usize>() {
            Ok(n) => {
                println!("Number of splits: {} from env", n);
                QuadWindowExpansionStrategy::Even(n)
            },
            Err(_) => {
                println!("Invalid number of splits: {}", s);
                QuadWindowExpansionStrategy::None
            },
        },
        Err(_) => match std::env::var("SUB_SPLITS_SPAN") {
            Ok(s) => match s.parse::<usize>() {
                Ok(n) => {
                    println!("Number of scans per split: {} from env", n);
                    QuadWindowExpansionStrategy::Uniform((n, n / 2))
                },
                Err(_) => {
                    println!("Invalid number of splits: {}", s);
                    QuadWindowExpansionStrategy::None
                },
            },
            Err(_) => QuadWindowExpansionStrategy::None,
        },
    };

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
                &splits,
            ) {
                assert!(
                    sws >= subwindow_scan_start,
                    "{} >= {} not true",
                    sws,
                    subwindow_scan_start
                );
                assert!(
                    swe <= subwindow_scan_end,
                    "{} <= {} not true",
                    swe,
                    subwindow_scan_end
                );
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
    println!(
        "Number of expanded quad settings {}",
        expanded_quadrupole_settings.len()
    );
    expanded_quadrupole_settings
}
