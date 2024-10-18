use std::{
    path::{Path, PathBuf},
    sync::Arc,
    vec,
};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    ms_data::{AcquisitionType, Frame, MSLevel, QuadrupoleSettings},
    utils::find_extension,
};

use super::{
    file_readers::{
        sql_reader::{
            frame_groups::SqlWindowGroup, frames::SqlFrame, ReadableSqlTable,
            SqlError, SqlReader,
        },
        tdf_blob_reader::{TdfBlob, TdfBlobReader, TdfBlobReaderError},
    },
    MetadataReader, MetadataReaderError, QuadrupoleSettingsReader,
    QuadrupoleSettingsReaderError,
};

#[derive(Debug)]
pub struct FrameReader {
    path: PathBuf,
    tdf_bin_reader: TdfBlobReader,
    frames: Vec<Frame>,
    acquisition: AcquisitionType,
    offsets: Vec<usize>,
    dia_windows: Option<Vec<Arc<QuadrupoleSettings>>>,
    compression_type: u8,
}

impl FrameReader {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, FrameReaderError> {
        let sql_path = find_extension(&path, "analysis.tdf").ok_or(
            FrameReaderError::FileNotFound("analysis.tdf".to_string()),
        )?;
        let compression_type =
            match MetadataReader::new(&sql_path)?.compression_type {
                2 => 2,
                compression_type => {
                    return Err(FrameReaderError::CompressionTypeError(
                        compression_type,
                    ))
                },
            };

        let tdf_sql_reader = SqlReader::open(sql_path)?;
        let sql_frames = SqlFrame::from_sql_reader(&tdf_sql_reader)?;
        let bin_path = find_extension(&path, "analysis.tdf_bin").ok_or(
            FrameReaderError::FileNotFound("analysis.tdf_bin".to_string()),
        )?;
        let tdf_bin_reader = TdfBlobReader::new(bin_path)?;
        let acquisition = if sql_frames.iter().any(|x| x.msms_type == 8) {
            AcquisitionType::DDAPASEF
        } else if sql_frames.iter().any(|x| x.msms_type == 9) {
            AcquisitionType::DIAPASEF
        } else {
            AcquisitionType::Unknown
        };
        // TODO should be refactored out to quadrupole reader
        let mut window_groups = vec![0; sql_frames.len()];
        let quadrupole_settings;
        if acquisition == AcquisitionType::DIAPASEF {
            for window_group in
                SqlWindowGroup::from_sql_reader(&tdf_sql_reader)?
            {
                window_groups[window_group.frame - 1] =
                    window_group.window_group;
            }
            quadrupole_settings =
                QuadrupoleSettingsReader::new(tdf_sql_reader.get_path())?;
        } else {
            quadrupole_settings = vec![];
        }
        let quadrupole_settings = quadrupole_settings
            .into_iter()
            .map(|x| Arc::new(x))
            .collect();
        let frames = (0..sql_frames.len())
            .into_par_iter()
            .map(|index| {
                get_frame_without_data(
                    index,
                    &sql_frames,
                    acquisition,
                    &window_groups,
                    &quadrupole_settings,
                )
            })
            .collect();
        let offsets = sql_frames.iter().map(|x| x.binary_offset).collect();
        let reader = Self {
            path: path.as_ref().to_path_buf(),
            tdf_bin_reader,
            frames,
            acquisition,
            offsets,
            dia_windows: match acquisition {
                AcquisitionType::DIAPASEF => Some(quadrupole_settings),
                _ => None,
            },
            compression_type,
        };
        Ok(reader)
    }

    pub fn parallel_filter<'a, F: Fn(&Frame) -> bool + Sync + Send + 'a>(
        &'a self,
        predicate: F,
    ) -> impl ParallelIterator<Item = Result<Frame, FrameReaderError>> + 'a
    {
        (0..self.len())
            .into_par_iter()
            .filter(move |x| predicate(&self.frames[*x]))
            .map(move |x| self.get(x))
    }

    pub fn filter<'a, F: Fn(&Frame) -> bool + Sync + Send + 'a>(
        &'a self,
        predicate: F,
    ) -> impl Iterator<Item = Result<Frame, FrameReaderError>> + 'a {
        (0..self.len())
            .filter(move |x| predicate(&self.frames[*x]))
            .map(move |x| self.get(x))
    }

    pub fn get_dia_windows(&self) -> Option<Vec<Arc<QuadrupoleSettings>>> {
        self.dia_windows.clone()
    }

    pub fn get(&self, index: usize) -> Result<Frame, FrameReaderError> {
        match self.compression_type {
            2 => self.get_from_compression_type_2(index),
            _ => Err(FrameReaderError::CompressionTypeError(
                self.compression_type,
            )),
        }
    }

    pub fn get_from_compression_type_2(
        &self,
        index: usize,
    ) -> Result<Frame, FrameReaderError> {
        // NOTE: get does it by 0-offsetting the vec, not by Frame index!!!
        let mut frame = self.get_frame_without_coordinates(index)?;
        let offset = self.offsets[index];
        let blob = self.tdf_bin_reader.get(offset)?;
        let scan_count: usize =
            blob.get(0).ok_or(FrameReaderError::CorruptFrame)? as usize;
        let peak_count: usize = (blob.len() - scan_count) / 2;
        frame.scan_offsets = read_scan_offsets(scan_count, peak_count, &blob)?;
        frame.intensities = read_intensities(scan_count, peak_count, &blob)?;
        frame.tof_indices = read_tof_indices(
            scan_count,
            peak_count,
            &blob,
            &frame.scan_offsets,
        )?;
        Ok(frame)
    }

    pub fn get_frame_without_coordinates(
        &self,
        index: usize,
    ) -> Result<Frame, FrameReaderError> {
        let frame = self
            .frames
            .get(index)
            .ok_or(FrameReaderError::IndexOutOfBounds)?
            .clone();
        Ok(frame)
    }

    pub fn get_all(&self) -> Vec<Result<Frame, FrameReaderError>> {
        self.parallel_filter(|_| true).collect()
    }

    pub fn get_all_ms1(&self) -> Vec<Result<Frame, FrameReaderError>> {
        self.parallel_filter(|x| x.ms_level == MSLevel::MS1)
            .collect()
    }

    pub fn get_all_ms2(&self) -> Vec<Result<Frame, FrameReaderError>> {
        self.parallel_filter(|x| x.ms_level == MSLevel::MS2)
            .collect()
    }

    pub fn get_acquisition(&self) -> AcquisitionType {
        self.acquisition
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}

fn read_scan_offsets(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Result<Vec<usize>, FrameReaderError> {
    let mut scan_offsets: Vec<usize> = Vec::with_capacity(scan_count + 1);
    scan_offsets.push(0);
    for scan_index in 0..scan_count - 1 {
        let index = scan_index + 1;
        let scan_size: usize =
            (blob.get(index).ok_or(FrameReaderError::CorruptFrame)? / 2)
                as usize;
        scan_offsets.push(scan_offsets[scan_index] + scan_size);
    }
    scan_offsets.push(peak_count);
    Ok(scan_offsets)
}

fn read_intensities(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Result<Vec<u32>, FrameReaderError> {
    let mut intensities: Vec<u32> = Vec::with_capacity(peak_count);
    for peak_index in 0..peak_count {
        let index: usize = scan_count + 1 + 2 * peak_index;
        intensities
            .push(blob.get(index).ok_or(FrameReaderError::CorruptFrame)?);
    }
    Ok(intensities)
}

fn read_tof_indices(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    scan_offsets: &Vec<usize>,
) -> Result<Vec<u32>, FrameReaderError> {
    let mut tof_indices: Vec<u32> = Vec::with_capacity(peak_count);
    for scan_index in 0..scan_count {
        let start_offset: usize = scan_offsets[scan_index];
        let end_offset: usize = scan_offsets[scan_index + 1];
        let mut current_sum: u32 = 0;
        for peak_index in start_offset..end_offset {
            let index = scan_count + 2 * peak_index;
            let tof_index: u32 =
                blob.get(index).ok_or(FrameReaderError::CorruptFrame)?;
            current_sum += tof_index;
            tof_indices.push(current_sum - 1);
        }
    }
    Ok(tof_indices)
}

fn get_frame_without_data(
    index: usize,
    sql_frames: &Vec<SqlFrame>,
    acquisition: AcquisitionType,
    window_groups: &Vec<u8>,
    quadrupole_settings: &Vec<Arc<QuadrupoleSettings>>,
) -> Frame {
    let mut frame: Frame = Frame::default();
    let sql_frame = &sql_frames[index];
    frame.index = sql_frame.id;
    frame.ms_level = MSLevel::read_from_msms_type(sql_frame.msms_type);
    frame.rt = sql_frame.rt;
    frame.acquisition_type = acquisition;
    frame.intensity_correction_factor = 1.0 / sql_frame.accumulation_time;
    if (acquisition == AcquisitionType::DIAPASEF)
        & (frame.ms_level == MSLevel::MS2)
    {
        // TODO should be refactored out to quadrupole reader
        let window_group = window_groups[index];
        frame.window_group = window_group;
        frame.quadrupole_settings =
            quadrupole_settings[window_group as usize - 1].clone();
    }
    frame
}

#[derive(Debug, thiserror::Error)]
pub enum FrameReaderError {
    #[error("{0}")]
    TdfBlobReaderError(#[from] TdfBlobReaderError),
    #[error("{0}")]
    FileNotFound(String),
    #[error("{0}")]
    SqlError(#[from] SqlError),
    #[error("Corrupt Frame")]
    CorruptFrame,
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
    #[error("Index out of bounds")]
    IndexOutOfBounds,
}
