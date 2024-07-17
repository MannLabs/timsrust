use std::path::Path;

use crate::{ms_data::QuadrupoleSettings, utils::vec_utils::argsort};

use super::file_readers::sql_reader::{
    quad_settings::SqlQuadSettings, ReadableSqlTable, SqlError, SqlReader,
};

pub struct QuadrupoleSettingsReader {
    quadrupole_settings: Vec<QuadrupoleSettings>,
    sql_quadrupole_settings: Vec<SqlQuadSettings>,
}

impl QuadrupoleSettingsReader {
    pub fn new(
        path: impl AsRef<Path>,
    ) -> Result<Vec<QuadrupoleSettings>, QuadrupoleSettingsReaderError> {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(&sql_path)?;
        let sql_quadrupole_settings =
            SqlQuadSettings::from_sql_reader(&tdf_sql_reader)?;
        let window_group_count = sql_quadrupole_settings
            .iter()
            .map(|x| x.window_group)
            .max()
            .unwrap() as usize; // SqlReader cannot return empty vecs, so always succeeds
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
    // #[error("{0}")]
    // MiniTDFPrecursorReaderError(#[from] MiniTDFPrecursorReaderError),
    // #[error("{0}")]
    // TDFPrecursorReaderError(#[from] TDFPrecursorReaderError),
    #[error("{0}")]
    SqlError(#[from] SqlError),
}
