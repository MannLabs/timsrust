use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter,
    },
    io::readers::{
        file_readers::sql_reader::{
            precursors::SqlPrecursor, ReadableSqlTable, SqlReader,
            SqlReaderError,
        },
        MetadataReader, MetadataReaderError,
    },
    ms_data::Precursor,
    readers::TimsTofPathLike,
};

use super::PrecursorReaderTrait;

#[derive(Debug)]
pub struct DDATDFPrecursorReader {
    sql_precursors: Vec<SqlPrecursor>,
    rt_converter: Frame2RtConverter,
    im_converter: Scan2ImConverter,
}

impl DDATDFPrecursorReader {
    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Self, DDATDFPrecursorReaderError> {
        let tdf_sql_reader = SqlReader::open(&path)?;
        let metadata = MetadataReader::new(&path)?;
        let rt_converter: Frame2RtConverter = metadata.rt_converter;
        let im_converter: Scan2ImConverter = metadata.im_converter;
        let sql_precursors = SqlPrecursor::from_sql_reader(&tdf_sql_reader)?;
        let reader = Self {
            sql_precursors,
            rt_converter,
            im_converter,
        };
        Ok(reader)
    }
}

impl PrecursorReaderTrait for DDATDFPrecursorReader {
    fn get(&self, index: usize) -> Option<Precursor> {
        let sql_precursor = &self.sql_precursors.get(index)?;
        let frame_id: usize = sql_precursor.precursor_frame;
        let scan_id: f64 = sql_precursor.scan_average;
        let precursor = Precursor {
            mz: sql_precursor.mz,
            rt: self.rt_converter.convert(frame_id as u32),
            im: self.im_converter.convert(scan_id),
            charge: Some(sql_precursor.charge),
            intensity: Some(sql_precursor.intensity),
            index: index + 1,
            frame_index: frame_id,
        };
        Some(precursor)
    }

    fn len(&self) -> usize {
        self.sql_precursors.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DDATDFPrecursorReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
}
