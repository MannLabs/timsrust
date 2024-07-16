use std::path::{Path, PathBuf};

use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter,
    },
    io::readers::{
        file_readers::sql_reader::{
            precursors::SqlPrecursor, ReadableSqlTable, SqlReader,
        },
        MetadataReader,
    },
    ms_data::Precursor,
};

use super::PrecursorReaderTrait;

#[derive(Debug)]
pub struct DDATDFPrecursorReader {
    path: PathBuf,
    sql_precursors: Vec<SqlPrecursor>,
    rt_converter: Frame2RtConverter,
    im_converter: Scan2ImConverter,
}

impl DDATDFPrecursorReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(sql_path).unwrap();
        let metadata = MetadataReader::new(&path).unwrap();
        let rt_converter: Frame2RtConverter = metadata.rt_converter;
        let im_converter: Scan2ImConverter = metadata.im_converter;
        let sql_precursors =
            SqlPrecursor::from_sql_reader(&tdf_sql_reader).unwrap();
        Self {
            path: path.as_ref().to_path_buf(),
            sql_precursors,
            rt_converter,
            im_converter,
        }
    }
}

impl PrecursorReaderTrait for DDATDFPrecursorReader {
    fn get(&self, index: usize) -> Precursor {
        let sql_precursor = &self.sql_precursors[index];
        let frame_id: usize = sql_precursor.precursor_frame;
        let scan_id: f64 = sql_precursor.scan_average;
        Precursor {
            mz: sql_precursor.mz,
            rt: self.rt_converter.convert(frame_id as u32),
            im: self.im_converter.convert(scan_id),
            charge: Some(sql_precursor.charge),
            intensity: Some(sql_precursor.intensity),
            index: index + 1,
            frame_index: frame_id,
        }
    }

    fn len(&self) -> usize {
        self.sql_precursors.len()
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}
