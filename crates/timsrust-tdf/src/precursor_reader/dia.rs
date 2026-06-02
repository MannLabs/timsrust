use std::sync::Arc;

use timsrust_core::{
    Converter, FrameIndex, Im, InvertibleConverter, Precursor,
    QuadrupoleSettings, ScanIndex, utils::reader::Reader,
};

use crate::{
    Frame2RtConverter, FrameWindowSplittingConfiguration, Metadata,
    MetadataReaderError, QuadrupoleSettingsReader,
    QuadrupoleSettingsReaderError, TDFPathLike,
    file_readers::sql_reader::{SqlReader, SqlReaderError},
};

#[derive(Debug)]
pub(crate) struct DIATDFPrecursorReader<ImC> {
    expanded_quadrupole_settings: Vec<QuadrupoleSettings>,
    rt_converter: Arc<Frame2RtConverter>,
    im_converter: Arc<ImC>,
}

impl<ImC: InvertibleConverter<ScanIndex, Im>> DIATDFPrecursorReader<ImC> {
    pub(crate) fn new(
        path: impl TDFPathLike,
        splitting_config: FrameWindowSplittingConfiguration<ImC>,
        im_converter: Arc<ImC>,
    ) -> Result<Self, DIATDFPrecursorReaderError> {
        let tdf_sql_reader = SqlReader::open(&path)?;
        let metadata = Metadata::new(&path)?;
        let rt_converter = metadata.rt_converter().clone();
        let splitting_strategy =
            splitting_config.finalize(Some(im_converter.clone()));
        let expanded_quadrupole_settings =
            QuadrupoleSettingsReader::from_splitting(
                &tdf_sql_reader,
                splitting_strategy,
            )?;
        Ok(Self {
            expanded_quadrupole_settings,
            rt_converter,
            im_converter,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.expanded_quadrupole_settings.len()
    }
}

impl<ImC: Converter<ScanIndex, Im>> Reader<Precursor>
    for DIATDFPrecursorReader<ImC>
{
    type Error = DIATDFPrecursorReaderError;
    fn get(&self, index: usize) -> Result<Precursor, Self::Error> {
        let quad_settings =
            &self.expanded_quadrupole_settings.get(index).ok_or(
                DIATDFPrecursorReaderError::NoQuadrupoleSettingsAtIndex(index),
            )?;
        let scan_id = (quad_settings.scan_starts[0]
            + quad_settings.scan_ends[0]) as f32
            / 2.0;
        let scan = ScanIndex::try_from(scan_id as u32).unwrap();
        let precursor = Precursor::new(
            quad_settings.isolation_windows[0].center(),
            self.im_converter.convert(scan),
            self.rt_converter.convert(
                FrameIndex::try_from(quad_settings.index as u32 - 1).unwrap(),
            ),
            scan,
            None,
            None,
            index,
            FrameIndex::try_from(quad_settings.index).unwrap(),
            // frame_index: FrameIndex::try_from(quad_settings.index - 1).unwrap(),
        );
        Ok(precursor)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DIATDFPrecursorReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
    #[error("No quadrupole settings at index {0}")]
    NoQuadrupoleSettingsAtIndex(usize),
}
