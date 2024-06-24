use std::path::{Path, PathBuf};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::ms_data::{AcquisitionType, Frame, MSLevel};

use super::file_readers::{
    sql_reader::{frames::SqlFrame, ReadableSqlTable, SqlReader},
    tdf_blobs::{TdfBlob, TdfBlobReader},
};

#[derive(Debug)]
pub struct FrameReader {
    path: PathBuf,
    tdf_bin_reader: TdfBlobReader,
    sql_frames: Vec<SqlFrame>,
    acquisition: AcquisitionType,
}

impl FrameReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let sql_path = path.as_ref().join("analysis.tdf");
        let tdf_sql_reader = SqlReader::open(sql_path).unwrap();
        let sql_frames = SqlFrame::from_sql_reader(&tdf_sql_reader).unwrap();
        let bin_path = path.as_ref().join("analysis.tdf_bin");
        let tdf_bin_reader: TdfBlobReader =
            TdfBlobReader::new(bin_path).unwrap();
        let acquisition = if sql_frames.iter().any(|x| x.msms_type == 8) {
            AcquisitionType::DDAPASEF
        } else if sql_frames.iter().any(|x| x.msms_type == 9) {
            AcquisitionType::DIAPASEF
        } else {
            AcquisitionType::Unknown
        };
        Self {
            path: path.as_ref().to_path_buf(),
            tdf_bin_reader,
            sql_frames,
            acquisition,
        }
    }

    pub fn parallel_filter<'a, F: Fn(&SqlFrame) -> bool + Sync + Send + 'a>(
        &'a self,
        predicate: F,
    ) -> impl ParallelIterator<Item = Frame> + 'a {
        (0..self.len())
            .into_par_iter()
            .filter(move |x| predicate(&self.sql_frames[*x]))
            .map(move |x| self.get(x))
    }

    pub fn get(&self, index: usize) -> Frame {
        let mut frame: Frame = Frame::default();
        let sql_frame = &self.sql_frames[index];
        let blob = self
            .tdf_bin_reader
            .get_blob(sql_frame.binary_offset)
            .unwrap();
        let scan_count: usize = blob.get(0) as usize;
        let peak_count: usize = (blob.len() - scan_count) / 2;
        frame.scan_offsets = read_scan_offsets(scan_count, peak_count, &blob);
        frame.intensities = read_intensities(scan_count, peak_count, &blob);
        frame.tof_indices = read_tof_indices(
            scan_count,
            peak_count,
            &blob,
            &frame.scan_offsets,
        );
        frame.ms_level = MSLevel::read_from_msms_type(sql_frame.msms_type);
        frame.index = sql_frame.id;
        frame.rt = sql_frame.rt;
        frame.acquisition_type = self.acquisition;
        frame.intensity_correction_factor = 1.0 / sql_frame.accumulation_time;
        frame
    }

    pub fn get_acquisition(&self) -> AcquisitionType {
        self.acquisition
    }

    pub fn len(&self) -> usize {
        self.sql_frames.len()
    }

    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}

fn read_scan_offsets(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Vec<usize> {
    let mut scan_offsets: Vec<usize> = Vec::with_capacity(scan_count + 1);
    scan_offsets.push(0);
    for scan_index in 0..scan_count - 1 {
        let index = scan_index + 1;
        let scan_size: usize = (blob.get(index) / 2) as usize;
        scan_offsets.push(scan_offsets[scan_index] + scan_size);
    }
    scan_offsets.push(peak_count);
    scan_offsets
}

fn read_intensities(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Vec<u32> {
    let mut intensities: Vec<u32> = Vec::with_capacity(peak_count);
    for peak_index in 0..peak_count {
        let index: usize = scan_count + 1 + 2 * peak_index;
        intensities.push(blob.get(index));
    }
    intensities
}

fn read_tof_indices(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    scan_offsets: &Vec<usize>,
) -> Vec<u32> {
    let mut tof_indices: Vec<u32> = Vec::with_capacity(peak_count);
    for scan_index in 0..scan_count {
        let start_offset: usize = scan_offsets[scan_index];
        let end_offset: usize = scan_offsets[scan_index + 1];
        let mut current_sum: u32 = 0;
        for peak_index in start_offset..end_offset {
            let index = scan_count + 2 * peak_index;
            let tof_index: u32 = blob.get(index);
            current_sum += tof_index;
            tof_indices.push(current_sum - 1);
        }
    }
    tof_indices
}
