#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

use crate::{
    domain_converters::{ConvertableDomain, Scan2ImConverter},
    ms_data::QuadrupoleSettings,
    utils::vec_utils::argsort,
};

use super::{
    file_readers::sql_reader::{
        frame_groups::SqlWindowGroup, quad_settings::SqlQuadSettings,
        ReadableSqlTable, SqlReader, SqlReaderError,
    },
    TimsTofPathLike,
};

pub struct QuadrupoleSettingsReader {
    quadrupole_settings: Vec<QuadrupoleSettings>,
    sql_quadrupole_settings: Vec<SqlQuadSettings>,
}

impl QuadrupoleSettingsReader {
    // TODO: refactor due to large size
    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Vec<QuadrupoleSettings>, QuadrupoleSettingsReaderError> {
        let tdf_sql_reader = SqlReader::open(path)?;
        Self::from_sql_settings(&tdf_sql_reader)
    }

    pub fn from_sql_settings(
        tdf_sql_reader: &SqlReader,
    ) -> Result<Vec<QuadrupoleSettings>, QuadrupoleSettingsReaderError> {
        let sql_quadrupole_settings =
            SqlQuadSettings::from_sql_reader(&tdf_sql_reader)?;
        let window_group_count = sql_quadrupole_settings
            .iter()
            .map(|x| x.window_group)
            .max()
            .expect("SqlReader cannot return empty vecs, so there is always a max window_group")
            as usize;
        let quadrupole_settings = (0..window_group_count)
            .map(|window_group| {
                let mut quad = QuadrupoleSettings::default();
                quad.index = window_group + 1;
                quad
            })
            .collect();
        let mut quad_reader = Self {
            quadrupole_settings,
            sql_quadrupole_settings,
        };
        quad_reader.update_from_sql_quadrupole_settings();
        quad_reader.resort_groups();
        Ok(quad_reader.quadrupole_settings)
    }

    pub fn from_splitting(
        tdf_sql_reader: &SqlReader,
        splitting_strat: FrameWindowSplittingStrategy,
    ) -> Result<Vec<QuadrupoleSettings>, QuadrupoleSettingsReaderError> {
        let quadrupole_settings = Self::from_sql_settings(&tdf_sql_reader)?;
        let window_groups = SqlWindowGroup::from_sql_reader(&tdf_sql_reader)?;
        let expanded_quadrupole_settings = match splitting_strat {
            FrameWindowSplittingStrategy::Quadrupole(x) => {
                expand_quadrupole_settings(
                    &window_groups,
                    &quadrupole_settings,
                    &x,
                )
            },
            FrameWindowSplittingStrategy::Window(x) => {
                expand_window_settings(&window_groups, &quadrupole_settings, &x)
            },
        };
        Ok(expanded_quadrupole_settings)
    }

    fn update_from_sql_quadrupole_settings(&mut self) {
        for window_group in self.sql_quadrupole_settings.iter() {
            let group = window_group.window_group - 1;
            self.quadrupole_settings[group]
                .scan_starts
                .push(window_group.scan_start);
            self.quadrupole_settings[group]
                .scan_ends
                .push(window_group.scan_end);
            self.quadrupole_settings[group]
                .collision_energy
                .push(window_group.collision_energy);
            self.quadrupole_settings[group]
                .isolation_mz
                .push(window_group.mz_center);
            self.quadrupole_settings[group]
                .isolation_width
                .push(window_group.mz_width);
        }
    }

    fn resort_groups(&mut self) {
        self.quadrupole_settings = self
            .quadrupole_settings
            .iter()
            .map(|_window| {
                let mut window = _window.clone();
                let order = argsort(&window.scan_starts);
                window.isolation_mz =
                    order.iter().map(|&i| window.isolation_mz[i]).collect();
                window.isolation_width =
                    order.iter().map(|&i| window.isolation_width[i]).collect();
                window.collision_energy =
                    order.iter().map(|&i| window.collision_energy[i]).collect();
                window.scan_starts =
                    order.iter().map(|&i| window.scan_starts[i]).collect();
                window.scan_ends =
                    order.iter().map(|&i| window.scan_ends[i]).collect();
                window
            })
            .collect();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QuadrupoleSettingsReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
}

type MobilitySpanStep = (f64, f64);
type ScanSpanStep = (usize, usize);

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
/// * `UniformMobility(SpanStep)` - Split the quadrupole settings into subwindows of
/// width `SpanStep.0` and step `SpanStep.1` in ion mobility space.
/// e.g. if `SpanStep` is (0.05, 0.02),
/// the window will be split into subwindows of width 0.05 and step 0.02 between their
/// in the mobility dimension.
/// * `UniformScan(SpanStep)` - Split the quadrupole settings into subwindows of
/// width `SpanStep.0` and step `SpanStep.1` in scan number space.
/// e.g. if `SpanStep` is (100, 80),
/// the window will be split into subwindows of width
/// 100 and step 80 between their in the scan number.
///
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub enum QuadWindowExpansionStrategy {
    None,
    Even(usize),
    UniformMobility(MobilitySpanStep, Option<Scan2ImConverter>),
    UniformScan(ScanSpanStep),
}

impl Default for QuadWindowExpansionStrategy {
    fn default() -> Self {
        Self::Even(1)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FrameWindowSplittingStrategy {
    Quadrupole(QuadWindowExpansionStrategy),
    Window(QuadWindowExpansionStrategy),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub enum FrameWindowSplittingConfiguration {
    Quadrupole(QuadWindowExpansionStrategy),
    Window(QuadWindowExpansionStrategy),
}

impl Default for FrameWindowSplittingConfiguration {
    fn default() -> Self {
        Self::Quadrupole(QuadWindowExpansionStrategy::Even(1))
    }
}

impl FrameWindowSplittingConfiguration {
    pub fn finalize(
        self,
        scan_converter: Option<Scan2ImConverter>,
    ) -> FrameWindowSplittingStrategy {
        match self {
            Self::Quadrupole(x) => FrameWindowSplittingStrategy::Quadrupole(
                Self::update_im_converter(x, scan_converter),
            ),
            Self::Window(x) => FrameWindowSplittingStrategy::Window(
                Self::update_im_converter(x, scan_converter),
            ),
        }
    }

    fn update_im_converter(
        quad_strategy: QuadWindowExpansionStrategy,
        scan_converter: Option<Scan2ImConverter>,
    ) -> QuadWindowExpansionStrategy {
        match quad_strategy {
            QuadWindowExpansionStrategy::UniformMobility((span, step), _) => {
                QuadWindowExpansionStrategy::UniformMobility(
                    (span, step),
                    scan_converter,
                )
            },
            _ => quad_strategy.clone(),
        }
    }
}

fn scan_range_subsplit(
    start: usize,
    end: usize,
    strategy: &QuadWindowExpansionStrategy,
) -> Vec<(usize, usize)> {
    let out: Vec<(usize, usize)> = match strategy {
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
        QuadWindowExpansionStrategy::UniformMobility(
            (span, step),
            _converter,
        ) => {
            // Since scan start < scan end but low scans are high IMs, we need to
            // subtract instead of adding.
            let converter = _converter.unwrap(); // Should always pass if created from FrameWindowConfig
            let mut curr_start_offset = start.clone();
            let mut curr_start_im = converter.convert(curr_start_offset as f64);

            let mut curr_end_im = curr_start_im - span;
            let mut curr_end_offset = converter.invert(curr_end_im) as usize;
            let mut out = Vec::new();
            while curr_end_offset < end {
                out.push((curr_start_offset, curr_end_offset));

                curr_start_im = curr_start_im - step;
                curr_start_offset = converter.invert(curr_start_im) as usize;

                curr_end_im = curr_start_im - span;
                curr_end_offset = converter.invert(curr_end_im) as usize;
            }
            if curr_start_offset < end {
                out.push((curr_start_offset, end));
            }
            out
        },
        QuadWindowExpansionStrategy::UniformScan((span, step)) => {
            let mut curr_start_offset = start;
            let mut curr_end_offset = start + span;
            let mut out = Vec::new();

            while curr_end_offset < end {
                out.push((curr_start_offset, curr_end_offset));
                curr_start_offset += step;
                curr_end_offset += step;
            }
            if curr_start_offset < end {
                out.push((curr_start_offset, end));
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

fn expand_window_settings(
    window_groups: &[SqlWindowGroup],
    quadrupole_settings: &[QuadrupoleSettings],
    strategy: &QuadWindowExpansionStrategy,
) -> Vec<QuadrupoleSettings> {
    let mut expanded_quadrupole_settings: Vec<QuadrupoleSettings> = vec![];
    for window_group in window_groups {
        let window = window_group.window_group;
        let frame = window_group.frame;
        let group = &quadrupole_settings[window as usize - 1];
        let window_group_start = group
            .scan_starts
            .iter()
            .min()
            .expect("SqlReader cannot return empty vecs, so there is always min window_group index")
            .clone();
        let window_group_end = group
            .scan_ends
            .iter()
            .max()
            .expect("SqlReader cannot return empty vecs, so there is always max window_group index")
            .clone();
        for (sws, swe) in
            scan_range_subsplit(window_group_start, window_group_end, &strategy)
        {
            let mut mz_min = std::f64::MAX;
            let mut mz_max = std::f64::MIN;
            let mut nce_sum = 0.0;
            let mut total_scan_width = 0.0;
            for i in 0..group.len() {
                let gss = group.scan_starts[i];
                let gse = group.scan_ends[i];
                if (swe <= gse) || (gss <= sws) {
                    continue;
                }
                let half_isolation_width = group.isolation_width[i] / 2.0;
                let isolation_mz = group.isolation_mz[i];
                mz_min = mz_min.min(isolation_mz - half_isolation_width);
                mz_max = mz_max.max(isolation_mz + half_isolation_width);
                let scan_width = (gse.min(swe) - gss.max(sws)) as f64;
                nce_sum += group.collision_energy[i] * scan_width;
                total_scan_width += scan_width
            }
            let sub_quad_settings = QuadrupoleSettings {
                index: frame,
                scan_starts: vec![sws],
                scan_ends: vec![swe],
                isolation_mz: vec![(mz_min + mz_max) / 2.0],
                isolation_width: vec![mz_min - mz_max],
                collision_energy: vec![nce_sum / total_scan_width],
            };
            expanded_quadrupole_settings.push(sub_quad_settings)
        }
    }
    expanded_quadrupole_settings
}

fn expand_quadrupole_settings(
    window_groups: &[SqlWindowGroup],
    quadrupole_settings: &[QuadrupoleSettings],
    strategy: &QuadWindowExpansionStrategy,
) -> Vec<QuadrupoleSettings> {
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
