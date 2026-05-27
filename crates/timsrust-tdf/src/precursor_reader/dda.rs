use std::sync::Arc;

use timsrust_core::{
    Charge, Converter, FrameIndex, Im, Mz, Precursor, ScanIndex,
    utils::reader::Reader,
};

use crate::{
    Frame2RtConverter, Metadata, MetadataReaderError, TDFPathLike,
    file_readers::sql_reader::{
        ReadableSqlTable, SqlReader, SqlReaderError, precursors::SqlPrecursor,
    },
};

#[derive(Debug)]
pub(crate) struct DDATDFPrecursorReader<ImC> {
    sql_precursors: Vec<SqlPrecursor>,
    rt_converter: Arc<Frame2RtConverter>,
    im_converter: Arc<ImC>,
}

impl<ImC: Converter<ScanIndex, Im>> DDATDFPrecursorReader<ImC> {
    pub(crate) fn new(
        path: impl TDFPathLike,
        im_converter: Arc<ImC>,
    ) -> Result<Self, DDATDFPrecursorReaderError> {
        let tdf_sql_reader = SqlReader::open(&path)?;
        let metadata = Metadata::new(&path)?;
        let rt_converter = metadata.rt_converter().clone();
        let sql_precursors = SqlPrecursor::from_sql_reader(&tdf_sql_reader)?;
        Ok(Self {
            sql_precursors,
            rt_converter,
            im_converter,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.sql_precursors.len()
    }
}

impl<ImC: Converter<ScanIndex, Im>> Reader<Precursor>
    for DDATDFPrecursorReader<ImC>
{
    type Error = DDATDFPrecursorReaderError;
    fn get(&self, index: usize) -> Result<Precursor, Self::Error> {
        let sql_precursor = &self
            .sql_precursors
            .get(index)
            .ok_or(DDATDFPrecursorReaderError::NoDataAtIndex(index))?;
        let frame_id: usize = sql_precursor.precursor_frame;
        let scan_id: f64 = sql_precursor.scan_average;
        let scan = ScanIndex::try_from(scan_id as u32).unwrap();
        let precursor = Precursor::new(
            Mz::from(sql_precursor.mz.unwrap_or_default()),
            self.im_converter.convert(scan),
            self.rt_converter
                .convert(FrameIndex::try_from(frame_id as u32).unwrap()),
            scan,
            sql_precursor.charge.map(|c| Charge::try_from(c).unwrap()),
            Some(sql_precursor.intensity),
            sql_precursor.id,
            FrameIndex::try_from(frame_id as u32).unwrap(),
        );
        Ok(precursor)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DDATDFPrecursorReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
    #[error("No data at index {0}")]
    NoDataAtIndex(usize),
}
