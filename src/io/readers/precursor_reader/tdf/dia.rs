use std::path::{Path, PathBuf};

use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter,
    },
    io::readers::{
        file_readers::sql_reader::{
            frame_groups::SqlWindowGroup, quad_settings::SqlQuadSettings,
            ReadableSqlTable, SqlReader,
        },
        MetadataReader,
    },
    ms_data::{Precursor, QuadrupoleSettings},
    utils::vec_utils::argsort,
};

use super::PrecursorReaderTrait;

#[derive(Debug)]
pub struct DIATDFPrecursorReader {
    path: PathBuf,
    expanded_quadrupole_settings: Vec<QuadrupoleSettings>,
    rt_converter: Frame2RtConverter,
    im_converter: Scan2ImConverter,
}

impl DIATDFPrecursorReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        // TODO: refactor or even better: recycle
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(sql_path).unwrap();
        let metadata = MetadataReader::new(&path);
        let rt_converter: Frame2RtConverter = metadata.rt_converter;
        let im_converter: Scan2ImConverter = metadata.im_converter;
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
            path: path.as_ref().to_path_buf(),
            expanded_quadrupole_settings,
            rt_converter,
            im_converter,
        }
    }
}

impl PrecursorReaderTrait for DIATDFPrecursorReader {
    fn get(&self, index: usize) -> Precursor {
        let quad_settings = &self.expanded_quadrupole_settings[index];
        let scan_id = (quad_settings.scan_starts[0]
            + quad_settings.scan_ends[0]) as f32
            / 2.0;
        Precursor {
            mz: quad_settings.isolation_mz[0],
            rt: self.rt_converter.convert(quad_settings.index as u32 - 1),
            im: self.im_converter.convert(scan_id),
            charge: 0,      //TODO
            intensity: 0.0, //TODO
            index: index,
            frame_index: quad_settings.index,
        }
    }

    fn len(&self) -> usize {
        self.expanded_quadrupole_settings.len()
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}
