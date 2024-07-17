use crate::{
    io::readers::{
        file_readers::sql_reader::{
            frame_groups::SqlWindowGroup, ReadableSqlTable, SqlError, SqlReader,
        },
        FrameReader, QuadrupoleSettingsReader, QuadrupoleSettingsReaderError,
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
    pub fn new(
        tdf_sql_reader: &SqlReader,
        frame_reader: FrameReader,
    ) -> Result<Self, DIARawSpectrumReaderError> {
        let window_groups = SqlWindowGroup::from_sql_reader(&tdf_sql_reader)?;
        let quadrupole_settings =
            QuadrupoleSettingsReader::new(&tdf_sql_reader.get_path())?;
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
        let reader = Self {
            expanded_quadrupole_settings,
            frame_reader,
        };
        Ok(reader)
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
        let frame = self.frame_reader.get(frame_index).unwrap();
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

#[derive(Debug, thiserror::Error)]
pub enum DIARawSpectrumReaderError {
    #[error("{0}")]
    SqlError(#[from] SqlError),
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
}
