use crate::{
    io::readers::{
        file_readers::sql_reader::{
            frame_groups::SqlWindowGroup, quad_settings::SqlQuadSettings,
            ReadableSqlTable, SqlReader,
        },
        FrameReader,
    },
    ms_data::QuadrupoleSettings,
    utils::vec_utils::{argsort, group_and_sum},
};

use super::raw_spectra::{RawSpectrum, RawSpectrumReaderTrait};

#[derive(Debug)]
pub struct DIARawSpectrumReader {
    expanded_quadrupole_settings: Vec<QuadrupoleSettings>,
    frame_reader: FrameReader,
}

impl DIARawSpectrumReader {
    pub fn new(tdf_sql_reader: &SqlReader, frame_reader: FrameReader) -> Self {
        let window_groups =
            SqlWindowGroup::from_sql_reader(&tdf_sql_reader).unwrap();
        let mut quadrupole_settings: Vec<QuadrupoleSettings>;
        let sql_quadrupole_settings =
            SqlQuadSettings::from_sql_reader(&tdf_sql_reader).unwrap();
        let window_group_count =
            window_groups.iter().map(|x| x.window_group).max().unwrap()
                as usize;
        quadrupole_settings = (0..window_group_count)
            .map(|window_group| {
                let mut quad = QuadrupoleSettings::default();
                quad.index = window_group + 1;
                quad
            })
            .collect();
        for window_group in sql_quadrupole_settings {
            let group = window_group.window_group - 1;
            quadrupole_settings[group]
                .scan_starts
                .push(window_group.scan_start);
            quadrupole_settings[group]
                .scan_ends
                .push(window_group.scan_end);
            quadrupole_settings[group]
                .collision_energy
                .push(window_group.collision_energy);
            quadrupole_settings[group]
                .isolation_mz
                .push(window_group.mz_center);
            quadrupole_settings[group]
                .isolation_width
                .push(window_group.mz_width);
        }
        quadrupole_settings = quadrupole_settings
            .into_iter()
            .map(|mut window| {
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
        let mut expanded_quadrupole_settings: Vec<QuadrupoleSettings> = vec![];
        for window_group in window_groups {
            let window = window_group.window_group;
            let frame = window_group.frame;
            let group = &quadrupole_settings[window as usize - 1];
            for sub_window in 0..group.isolation_mz.len() {
                let sub_quad_settings = QuadrupoleSettings {
                    index: frame,
                    scan_starts: vec![group.scan_starts[sub_window]],
                    scan_ends: vec![group.scan_ends[sub_window]],
                    isolation_mz: vec![group.isolation_mz[sub_window]],
                    isolation_width: vec![group.isolation_width[sub_window]],
                    collision_energy: vec![group.collision_energy[sub_window]],
                };
                expanded_quadrupole_settings.push(sub_quad_settings)
            }
        }
        Self {
            expanded_quadrupole_settings,
            frame_reader,
        }
    }
}

impl RawSpectrumReaderTrait for DIARawSpectrumReader {
    fn get(&self, index: usize) -> RawSpectrum {
        let quad_settings = &self.expanded_quadrupole_settings[index];
        let collision_energy = quad_settings.collision_energy[0];
        let isolation_mz = quad_settings.isolation_mz[0];
        let isolation_width = quad_settings.isolation_width[0];
        let scan_start = quad_settings.scan_starts[0];
        let scan_end = quad_settings.scan_ends[0];
        let frame_index = quad_settings.index - 1;
        let frame = self.frame_reader.get(frame_index);
        let offset_start = frame.scan_offsets[scan_start] as usize;
        let offset_end = frame.scan_offsets[scan_end] as usize;
        let tof_indices = &frame.tof_indices[offset_start..offset_end];
        let intensities = &frame.intensities[offset_start..offset_end];
        let (raw_tof_indices, raw_intensities) = group_and_sum(
            tof_indices.iter().map(|x| *x).collect(),
            intensities.iter().map(|x| *x as u64).collect(),
        );
        let raw_spectrum = RawSpectrum {
            tof_indices: raw_tof_indices,
            intensities: raw_intensities,
            index: index,
            collision_energy,
            isolation_mz,
            isolation_width,
        };
        raw_spectrum
    }
}
