use std::sync::Arc;

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, ParallelIterator,
};
#[cfg(feature = "timscompress")]
use timscompress::reader::CompressedTdfBlobReader;

use crate::{ms_data::{
    AcquisitionType, Frame, FrameMeta, FramePeaks, MSLevel, Metadata, MetadataReaderError, QuadrupoleSettings
}, FrameCalibration, WindowGroupInfo};

use super::{
    file_readers::{
        sql_reader::{
            frame_groups::SqlWindowGroup, frames::SqlFrame, ReadableSqlTable,
            SqlReader, SqlReaderError,
        },
        tdf_blob_reader::{TdfBlobReader, TdfBlobReaderError},
    },
     QuadrupoleSettingsReader,
    QuadrupoleSettingsReaderError, TimsTofPathLike,
};

// This is just a re-expport so users can create buffers
// to usethe buffered read
pub use super::file_readers::tdf_blob_reader::TdfBlob;

#[derive(Debug)]
pub struct FrameReader {
    tdf_bin_reader: TdfBlobReader,
    #[cfg(feature = "timscompress")]
    compressed_reader: CompressedTdfBlobReader,
    offsets: Vec<usize>,
    pub dia_windows: Option<Vec<Arc<QuadrupoleSettings>>>,
    pub frame_metas: Vec<FrameMeta>,
    pub acquisition: AcquisitionType,
    pub compression_type: u8,
    #[cfg(feature = "timscompress")]
    scan_count: usize,
}

impl FrameReader {
    pub fn new(path: impl TimsTofPathLike) -> Result<Self, FrameReaderError> {
        let compression_type =
            match Metadata::new(&path)?.compression_type {
                2 => 2,
                #[cfg(feature = "timscompress")]
                3 => 3,
                compression_type => {
                    return Err(FrameReaderError::CompressionTypeError(
                        compression_type,
                    ))
                },
            };

        let tdf_sql_reader = SqlReader::open(&path)?;
        let sql_frames = SqlFrame::from_sql_reader(&tdf_sql_reader)?;
        let tdf_bin_reader = TdfBlobReader::new(&path)?;
        #[cfg(feature = "timscompress")]
        let compressed_reader = CompressedTdfBlobReader::new(&path)
            .ok_or_else(|| FrameReaderError::TimscompressError)?;
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
            quadrupole_settings = QuadrupoleSettingsReader::new(&path)?;
        } else {
            quadrupole_settings = vec![];
        }
        // TODO move Arc to quad settings reader?
        let quadrupole_settings = quadrupole_settings
            .into_iter()
            .map(|x| Arc::new(x))
            .collect();
        let frame_metas = (0..sql_frames.len())
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
        #[cfg(feature = "timscompress")]
        let scan_count = sql_frames
            .iter()
            .map(|frame| frame.scan_count)
            .max()
            .expect("Frame table cannot be empty")
            as usize;
        let offsets = sql_frames.iter().map(|x| x.binary_offset).collect();
        let reader = Self {
            tdf_bin_reader,
            frame_metas,
            acquisition,
            offsets,
            dia_windows: match acquisition {
                AcquisitionType::DIAPASEF => Some(quadrupole_settings),
                _ => None,
            },
            compression_type,
            #[cfg(feature = "timscompress")]
            compressed_reader,
            #[cfg(feature = "timscompress")]
            scan_count,
        };
        Ok(reader)
    }

    // TODO make option result
    pub fn get_binary_offset(&self, index: usize) -> usize {
        self.offsets[index]
    }


    /// Filters frames in parallel using the provided predicate function.
    /// and returns an iterator over the results.
    pub fn parallel_filter<'a, F: Fn(&FrameMeta) -> bool + Sync + Send + 'a>(
        &'a self,
        predicate: F,
    ) -> impl ParallelIterator<Item = Result<Frame, FrameReaderError>> + 'a
    {
        (0..self.len())
            .into_par_iter()
            .filter(move |x| predicate(&self.frame_metas[*x]))
            .map(move |x| self.get_by_internal_index(x))
    }

    pub fn filter<'a, F: Fn(&FrameMeta) -> bool + Sync + Send + 'a>(
        &'a self,
        predicate: F,
    ) -> impl Iterator<Item = Result<Frame, FrameReaderError>> + 'a {
        (0..self.len())
            .filter(move |x| predicate(&self.frame_metas[*x]))
            .map(move |x| self.get_by_internal_index(x))
    }

    pub fn get_dia_windows(&self) -> Option<Vec<Arc<QuadrupoleSettings>>> {
        self.dia_windows.clone()
    }

    /// Attempts to find the frame using the instrument index and
    /// returns it if found.
    pub fn get_by_frame_index(&self, frame_index: usize) -> Result<Frame, FrameReaderError> {
        let internal_index = self
            .frame_metas
            .binary_search_by_key(
                &frame_index, |x|x.index
            );

        match internal_index {
            Ok(index) => self.get_by_internal_index(index),
            Err(_) => Err(FrameReaderError::IndexOutOfBounds)
                    }
    }

    /// Gets a frame by the internal index within this data structure.
    pub fn get_by_internal_index(&self, index: usize) -> Result<Frame, FrameReaderError> {
        match self.compression_type {
            2 => self.get_from_compression_type_2(index),
            #[cfg(feature = "timscompress")]
            3 => self.get_from_compression_type_3(index),
            _ => Err(FrameReaderError::CompressionTypeError(
                self.compression_type,
            )),
        }
    }

    /// Fills the provided buffer with the frame data at the specified index.
    pub fn get_buffered(
        &self,
        index: usize,
        frame_buffer: &mut Frame,
        blob_buffer: &mut TdfBlob,
    ) -> Result<(), FrameReaderError> {
        frame_buffer.peaks.clear();
        match self.compression_type {
            2 => self.get_from_compression_type_2_to(index, frame_buffer, blob_buffer),
            // #[cfg(feature = "timscompress")]
            // 3 => self.get_from_jompression_type_3(index),
            _ => Err(FrameReaderError::CompressionTypeError(
                self.compression_type,
            )),
        }
    }

    fn get_from_compression_type_2_to(
        &self,
        index: usize,
        frame_buffer: &mut Frame,
        blob_buffer: &mut TdfBlob,
    ) -> Result<(), FrameReaderError> {
        // NOTE: get does it by 0-offsetting the vec, not by Frame index!!!
        let frame_meta = self.get_frame_without_coordinates(index)?;
        let offset = self.get_binary_offset(index);
        // This call allocates a new vec for the peaks
        // that gets discarded after use.
        // TODO: optimize if needed.
        self.tdf_bin_reader.get_into(offset, blob_buffer)?;
        let scan_count: usize =
            blob_buffer.get(0).ok_or(FrameReaderError::CorruptFrame)? as usize;
        let peak_count: usize = (blob_buffer.len() - scan_count) / 2;

        transfer_frame_meta(&frame_meta, &mut frame_buffer.meta);
        fill_peaks(scan_count, peak_count, &blob_buffer, &mut frame_buffer.peaks)?;
        Ok(())
    }

    fn get_from_compression_type_2(
        &self,
        index: usize,
    ) -> Result<Frame, FrameReaderError> {
        let mut out = Frame::default();
        let mut blob_buffer = TdfBlob::new_empty();
        self.get_from_compression_type_2_to(index, &mut out, &mut blob_buffer)?;
        Ok(out)
    }

    #[cfg(feature = "timscompress")]
    fn get_from_jompression_type_3(
        &self,
        index: usize,
    ) -> Result<Frame, FrameReaderError> {
        // NOTE: get does it by 0-offsetting the vec, not by Frame index!!!
        // TODO
        let mut frame_meta = self.get_frame_without_coordinates(index)?;
        let offset = self.get_binary_offset(index);
        let raw_frame = self
            .compressed_reader
            .get_raw_frame_data(offset, self.scan_count);
        let peaks = FramePeaks {
            tof_indices: raw_frame.tof_indices,
            intensities: raw_frame.intensities,
            scan_offsets: raw_frame.scan_offsets,
        };
        Ok(Frame {
            peaks,
            meta: frame_meta,
        })
    }

    pub fn get_frame_without_coordinates(
        &self,
        index: usize,
    ) -> Result<FrameMeta, FrameReaderError> {
        let frame = self
            .frame_metas
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
        self.frame_metas.len()
    }
}

fn transfer_frame_meta(source: &FrameMeta, target: &mut FrameMeta) {
    // Using destructuring to make sure all fields are copied
    let FrameMeta {
        index,
        rt_in_seconds,
        acquisition_type,
        ms_level,
        intensity_correction_factor,
        window_group,
        calibration,
    } = source;
    target.index = *index;
    target.rt_in_seconds = *rt_in_seconds;
    target.acquisition_type = *acquisition_type;
    target.ms_level = *ms_level;
    target.window_group = window_group.clone();
    target.calibration = calibration.clone();
    target.intensity_correction_factor = *intensity_correction_factor;
}

fn fill_peaks(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    peak_buffer: &mut FramePeaks,
) -> Result<(), FrameReaderError> {
    read_scan_offsets_to(
        scan_count,
        peak_count,
        &blob,
        &mut peak_buffer.scan_offsets,
    )?;
    read_intensities_to(
        scan_count,
        peak_count,
        &blob,
        &mut peak_buffer.intensities,
    )?;
    read_tof_indices_to(
        scan_count,
        peak_count,
        &blob,
        &*peak_buffer.scan_offsets,
        &mut peak_buffer.tof_indices,
    )?;
    Ok(())
}

fn read_scan_offsets(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Result<Vec<u32>, FrameReaderError> {
    // I am making explicit the offsets to be u32 for memory, since
    // in 64 bit systems usize is 64 bit and this would double the memory
    // and I am expecting these to be always smaller than 4 billion.
    let mut scan_offsets: Vec<u32> = Vec::with_capacity(scan_count + 1);
    read_scan_offsets_to(scan_count, peak_count, blob, &mut scan_offsets)?;

    Ok(scan_offsets)
}

fn read_scan_offsets_to(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    offset_vec: &mut Vec<u32>,
) -> Result<(), FrameReaderError> {
    assert!(offset_vec.is_empty());
    offset_vec.reserve(scan_count + 1);

    offset_vec.push(0);
    let mut last_offset: u32 = 0;
    for scan_index in 0..scan_count - 1 {
        let index = scan_index + 1;
        let scan_size: u32 =
            blob.get(index).ok_or(FrameReaderError::CorruptFrame)? / 2;
        offset_vec.push(last_offset + scan_size);
        last_offset += scan_size;
    }
    offset_vec.push(peak_count.try_into().expect("Too many peaks"));
    Ok(())
}

fn read_intensities(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Result<Vec<u32>, FrameReaderError> {
    let mut intensities: Vec<u32> = Vec::with_capacity(peak_count);
    read_intensities_to(scan_count, peak_count, blob, &mut intensities)?;
    Ok(intensities)
}

fn read_intensities_to(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    intensities: &mut Vec<u32>,
) -> Result<(), FrameReaderError> {
    assert!(intensities.is_empty());
    intensities.reserve(peak_count);

    for peak_index in 0..peak_count {
        let index: usize = scan_count + 1 + 2 * peak_index;
        intensities
            .push(blob.get(index).ok_or(FrameReaderError::CorruptFrame)?);
    }
    Ok(())
}

fn read_tof_indices(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    scan_offsets: &[u32],
) -> Result<Vec<u32>, FrameReaderError> {
    let mut tof_indices: Vec<u32> = Vec::with_capacity(peak_count);
    read_tof_indices_to(
        scan_count,
        peak_count,
        blob,
        scan_offsets,
        &mut tof_indices,
    )?;
    Ok(tof_indices)
}

fn read_tof_indices_to(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    scan_offsets: &[u32],
    tof_indices: &mut Vec<u32>,
) -> Result<(), FrameReaderError> {
    assert!(tof_indices.is_empty());
    tof_indices.reserve(peak_count);

    for scan_index in 0..scan_count {
        let start_offset: usize = scan_offsets[scan_index] as usize;
        let end_offset: usize = scan_offsets[scan_index + 1] as usize;
        let mut current_sum: u32 = 0;
        for peak_index in start_offset..end_offset {
            let index = scan_count + 2 * peak_index;
            let tof_index: u32 =
                blob.get(index).ok_or(FrameReaderError::CorruptFrame)?;
            current_sum += tof_index;
            tof_indices.push(current_sum - 1);
        }
    }
    Ok(())
}

fn get_frame_without_data(
    index: usize,
    sql_frames: &Vec<SqlFrame>,
    acquisition: AcquisitionType,
    window_groups: &Vec<u8>,
    quadrupole_settings: &Vec<Arc<QuadrupoleSettings>>,
) -> FrameMeta {
    let sql_frame = &sql_frames[index];
    let mut frame: FrameMeta = FrameMeta {
        index: sql_frame.id,
        ms_level: MSLevel::read_from_msms_type(sql_frame.msms_type),
        rt_in_seconds: sql_frame.rt,
        acquisition_type: acquisition,
        // Since the correction factor is in essence the inverse of the accumulation time
        // meaning that for an intensity I and accumulation time t the corrected intensity
        // of I * (1/t) == (I/2) * (1/ (2 * t)).
        // Nontheless, we use 1000 instead of 1 to assure the corrected intensities are > 1
        intensity_correction_factor: 1000.0 / sql_frame.accumulation_time,
        calibration: FrameCalibration {
            calibration_id: sql_frame.mz_calibration,
            t1: sql_frame.t1,
            t2: sql_frame.t2,
        },
        window_group: None,
    };

    assert!(frame.intensity_correction_factor.is_finite());
    assert!(frame.intensity_correction_factor >= 1.0);

    if (acquisition == AcquisitionType::DIAPASEF)
        & (frame.ms_level == MSLevel::MS2)
    {
        // TODO should be refactored out to quadrupole reader
        let window_group = window_groups[index];
        frame.window_group = Some(WindowGroupInfo {
            window_group,
            quadrupole_settings: quadrupole_settings[window_group as usize - 1].clone(),
        });
    }
    frame
}

#[derive(Debug, thiserror::Error)]
pub enum FrameReaderError {
    #[cfg(feature = "timscompress")]
    #[error("Timscompress error")]
    TimscompressError,
    #[error("{0}")]
    TdfBlobReaderError(#[from] TdfBlobReaderError),
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    FileNotFound(String),
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("Corrupt Frame")]
    CorruptFrame,
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
    #[error("Index out of bounds")]
    IndexOutOfBounds,
    #[error("Compression type {0} not understood")]
    CompressionTypeError(u8),
}
