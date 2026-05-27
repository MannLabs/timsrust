use std::{collections::HashMap, sync::Arc};

use timsrust_core::{
    AcquisitionType, FrameInfo, MSLevel, QuadrupoleSettings,
    utils::reader::{IndexedReader, Reader},
};

use crate::{
    QuadrupoleSettingsReader, QuadrupoleSettingsReaderError, TDFPathLike,
    file_readers::sql_reader::{
        ReadableSqlTable, SqlReader, SqlReaderError,
        frame_groups::SqlWindowGroup, frames::SqlFrame,
    },
};

#[derive(Debug)]
pub struct FrameInfoReader {
    frame_infos: HashMap<usize, FrameInfo>,
    offsets: HashMap<usize, usize>,
    acquisition: AcquisitionType,
}

impl FrameInfoReader {
    pub fn new(
        path: impl TDFPathLike,
    ) -> Result<Self, FrameReaderErrorInternal> {
        let tdf_sql_reader = SqlReader::open(&path)?;
        let sql_frames = SqlFrame::from_sql_reader(&tdf_sql_reader)?;
        let acquisition = if sql_frames.iter().any(|x| x.msms_type == 8) {
            AcquisitionType::DDAPASEF
        } else if sql_frames.iter().any(|x| x.msms_type == 9) {
            AcquisitionType::DIAPASEF
        } else {
            AcquisitionType::Unknown
        };
        let mut window_groups = vec![0; sql_frames.len()];
        let quadrupole_settings;
        if acquisition == AcquisitionType::DIAPASEF {
            for window_group in
                SqlWindowGroup::from_sql_reader(&tdf_sql_reader)?
            {
                window_groups[window_group.frame - 1] =
                    window_group.window_group;
            }
            quadrupole_settings = QuadrupoleSettingsReader::new(&path)?;
        } else {
            quadrupole_settings = vec![];
        }
        let quadrupole_settings = quadrupole_settings
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>();
        let mut offsets = HashMap::new();
        let frame_infos = sql_frames
            .into_iter()
            .enumerate()
            .map(|(index, sql_frame)| {
                offsets.insert(sql_frame.id, sql_frame.binary_offset);
                let mut frame_info = FrameInfo::from(sql_frame);
                let cycle_index = if acquisition == AcquisitionType::DIAPASEF {
                    Some(index / quadrupole_settings.len())
                } else {
                    None
                };
                // dbg!(cycle_index);
                let (window_group, quad_settings) = if (acquisition
                    == AcquisitionType::DIAPASEF)
                    & (frame_info.ms_level() == MSLevel::MS2)
                {
                    // TODO should be refactored out to quadrupole reader
                    let window_group = window_groups[index];
                    let quad_settings =
                        quadrupole_settings[window_group as usize - 1].clone();
                    (window_group as u8, quad_settings)
                } else {
                    (
                        frame_info.window_group(),
                        frame_info.quadrupole_settings().clone(),
                    )
                };
                frame_info = FrameInfo::new(
                    quad_settings,
                    frame_info.index(),
                    frame_info.rt_in_seconds(),
                    frame_info.intensity_correction_factor(),
                    acquisition,
                    frame_info.ms_level(),
                    window_group,
                    cycle_index,
                );
                (frame_info.index(), frame_info)
            })
            .collect::<HashMap<_, _>>();
        let reader = Self {
            frame_infos,
            offsets,
            acquisition,
        };
        Ok(reader)
    }

    // pub(crate) fn get_offset(
    //     &self,
    //     index: usize,
    // ) -> Result<usize, FrameReaderErrorInternal> {
    //     self.offsets
    //         .get(&index)
    //         .cloned()
    //         .ok_or(FrameReaderErrorInternal::IndexOutOfBounds(index))
    // }

    pub fn get_acquisition(&self) -> AcquisitionType {
        self.acquisition
    }
}

impl From<SqlFrame> for FrameInfo {
    fn from(sql_frame: SqlFrame) -> Self {
        FrameInfo::new(
            Arc::new(QuadrupoleSettings::default()),
            sql_frame.id,
            sql_frame.rt,
            1000.0 / sql_frame.accumulation_time,
            AcquisitionType::Unknown,
            MSLevel::read_from_msms_type(sql_frame.msms_type),
            0,
            None,
        )
    }
}

impl Reader<FrameInfo> for FrameInfoReader {
    type Error = FrameReaderErrorInternal;

    fn get(
        &self,
        index: usize,
    ) -> Result<timsrust_core::FrameInfo, Self::Error> {
        self.frame_infos
            .get(&index)
            .cloned()
            .ok_or(FrameReaderErrorInternal::IndexOutOfBounds(index))
    }
}

impl FrameInfoReader {
    pub(crate) fn offsets_map(
        &self,
    ) -> std::collections::HashMap<usize, usize> {
        self.offsets.clone()
    }
}

impl IndexedReader<FrameInfo> for FrameInfoReader {
    type Iter = std::vec::IntoIter<usize>;
    fn iter(&self) -> Self::Iter {
        self.frame_infos
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[allow(private_interfaces)]
#[derive(Debug, thiserror::Error)]
pub enum FrameReaderErrorInternal {
    #[error("{0}")]
    FileNotFound(String),
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("Corrupt Frame")]
    CorruptFrame,
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
    #[error("Index out of bounds {0}")]
    IndexOutOfBounds(usize),
    #[error("Compression type {0} not understood")]
    CompressionTypeError(u8),
}
