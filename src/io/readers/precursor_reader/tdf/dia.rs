use std::path::Path;

use crate::io::readers::tdf_utils::{
    expand_quadrupole_settings, expand_window_settings,
};
use crate::io::readers::FrameWindowSplittingStrategy;
use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter,
    },
    io::readers::{
        file_readers::sql_reader::{
            frame_groups::SqlWindowGroup, ReadableSqlTable, SqlError, SqlReader,
        },
        MetadataReader, MetadataReaderError, QuadrupoleSettingsReader,
        QuadrupoleSettingsReaderError,
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
        splitting_strat: FrameWindowSplittingStrategy,
    ) -> Result<Self, DIATDFPrecursorReaderError> {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(sql_path)?;
        let metadata = MetadataReader::new(&path)?;
        let rt_converter: Frame2RtConverter = metadata.rt_converter;
        let im_converter: Scan2ImConverter = metadata.im_converter;
        let window_groups = SqlWindowGroup::from_sql_reader(&tdf_sql_reader)?;
        let quadrupole_settings =
            QuadrupoleSettingsReader::new(tdf_sql_reader.get_path())?;
        let expanded_quadrupole_settings = match splitting_strat {
            FrameWindowSplittingStrategy::None => quadrupole_settings,
            FrameWindowSplittingStrategy::Quadrupole(x) => {
                expand_quadrupole_settings(
                    &window_groups,
                    &quadrupole_settings,
                    &x,
                )
            },
            FrameWindowSplittingStrategy::Window(x) => {
                expand_window_settings(&window_groups, &quadrupole_settings, &x)
            },
        };

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
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
}
