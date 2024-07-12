use crate::io::readers::tdf_utils::expand_quadrupole_settings;
use crate::{
    io::readers::{
        file_readers::sql_reader::{
            frame_groups::SqlWindowGroup, ReadableSqlTable, SqlReader,
        },
        FrameReader, QuadrupoleSettingsReader,
    },
    ms_data::QuadrupoleSettings,
    utils::vec_utils::group_and_sum,
};

use super::raw_spectra::{RawSpectrum, RawSpectrumReaderTrait};

#[derive(Debug)]
pub struct DIARawSpectrumReader {
    expanded_quadrupole_settings: Vec<QuadrupoleSettings>,
    frame_reader: FrameReader,
}

impl DIARawSpectrumReader {
    pub fn new(tdf_sql_reader: &SqlReader, frame_reader: FrameReader) -> Self {
        let window_groups: Vec<SqlWindowGroup> =
            SqlWindowGroup::from_sql_reader(&tdf_sql_reader).unwrap();
        let quadrupole_settings =
            QuadrupoleSettingsReader::new(&tdf_sql_reader.get_path());
        let expanded_quadrupole_settings =
            expand_quadrupole_settings(&window_groups, &quadrupole_settings);
        Self {
            expanded_quadrupole_settings,
            frame_reader,
        }
    }
}

impl RawSpectrumReaderTrait for DIARawSpectrumReader {
    fn get(&self, index: usize) -> RawSpectrum {
        let quad_settings = &self.expanded_quadrupole_settings[index];
        if index < 10 {
            println!("{}", index);
            println!("{:?}", quad_settings);
        }
        if index > (self.expanded_quadrupole_settings.len() - 10) {
            println!("{}", index);
            println!("{:?}", quad_settings);
        }

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

    fn len(&self) -> usize {
        self.expanded_quadrupole_settings.len()
    }
}
