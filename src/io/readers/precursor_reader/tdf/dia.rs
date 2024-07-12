use std::path::{Path, PathBuf};

use crate::io::readers::tdf_utils::expand_quadrupole_settings;
use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter,
    },
    io::readers::{
        file_readers::sql_reader::{
            frame_groups::SqlWindowGroup, ReadableSqlTable, SqlReader,
        },
        MetadataReader, QuadrupoleSettingsReader,
    },
    ms_data::{Precursor, QuadrupoleSettings},
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
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(sql_path).unwrap();
        let metadata = MetadataReader::new(&path);
        let rt_converter: Frame2RtConverter = metadata.rt_converter;
        let im_converter: Scan2ImConverter = metadata.im_converter;
        let window_groups =
            SqlWindowGroup::from_sql_reader(&tdf_sql_reader).unwrap();
        let quadrupole_settings =
            QuadrupoleSettingsReader::new(tdf_sql_reader.get_path());
        let expanded_quadrupole_settings =
            expand_quadrupole_settings(&window_groups, &quadrupole_settings);
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
            charge: None,
            intensity: None,
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
