use std::path::Path;

use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter,
    },
    io::readers::{
        file_readers::sql_reader::{
            frame_groups::SqlWindowGroup, ReadableSqlTable, SqlError, SqlReader,
        },
        MetadataReader, MetadataReaderError, QuadrupoleSettingsReader,
    },
    ms_data::{Precursor, QuadrupoleSettings},
};

use super::PrecursorReaderTrait;

#[derive(Debug)]
pub struct DIATDFPrecursorReader {
    expanded_quadrupole_settings: Vec<QuadrupoleSettings>,
    rt_converter: Frame2RtConverter,
    im_converter: Scan2ImConverter,
}

impl DIATDFPrecursorReader {
    pub fn new(
        path: impl AsRef<Path>,
    ) -> Result<Self, DIATDFPrecursorReaderError> {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(sql_path)?;
        let metadata = MetadataReader::new(&path)?;
        let rt_converter: Frame2RtConverter = metadata.rt_converter;
        let im_converter: Scan2ImConverter = metadata.im_converter;
        let window_groups = SqlWindowGroup::from_sql_reader(&tdf_sql_reader)?;
        let quadrupole_settings =
            QuadrupoleSettingsReader::new(tdf_sql_reader.get_path());
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
            rt_converter,
            im_converter,
        };
        Ok(reader)
    }
}

impl PrecursorReaderTrait for DIATDFPrecursorReader {
    fn get(&self, index: usize) -> Option<Precursor> {
        let quad_settings = &self.expanded_quadrupole_settings.get(index)?;
        let scan_id = (quad_settings.scan_starts[0]
            + quad_settings.scan_ends[0]) as f32
            / 2.0;
        let precursor = Precursor {
            mz: quad_settings.isolation_mz[0],
            rt: self.rt_converter.convert(quad_settings.index as u32 - 1),
            im: self.im_converter.convert(scan_id),
            charge: None,
            intensity: None,
            index: index,
            frame_index: quad_settings.index,
        };
        Some(precursor)
    }

    fn len(&self) -> usize {
        self.expanded_quadrupole_settings.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DIATDFPrecursorReaderError {
    #[error("{0}")]
    SqlError(#[from] SqlError),
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
}
