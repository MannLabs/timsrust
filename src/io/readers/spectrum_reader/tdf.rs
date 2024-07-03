use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};

use crate::{
    domain_converters::{ConvertableDomain, Tof2MzConverter},
    io::readers::{
        file_readers::sql_reader::{
            pasef_frame_msms::SqlPasefFrameMsMs, ReadableSqlTable, SqlReader,
        },
        FrameReader, MetadataReader, PrecursorReader,
    },
    ms_data::{Precursor, Spectrum},
    utils::{
        find_extension,
        vec_utils::{
            argsort, filter_with_mask, find_sparse_local_maxima_mask,
            group_and_sum,
        },
    },
};

use super::SpectrumReaderTrait;

const SMOOTHING_WINDOW: u32 = 1;
const CENTROIDING_WINDOW: u32 = 1;
const CALIBRATION_TOLERANCE: f64 = 0.1;

#[derive(Debug)]
pub struct TDFSpectrumReader {
    path: PathBuf,
    precursor_reader: PrecursorReader,
    mz_reader: Tof2MzConverter,
    frame_reader: FrameReader,
    spectrum_frame_index_reader: SpectrumFrameIndexReader,
}

impl TDFSpectrumReader {
    pub fn new(path_name: impl AsRef<Path>) -> Self {
        let frame_reader: FrameReader = FrameReader::new(&path_name);
        let sql_path = find_extension(&path_name, "analysis.tdf").unwrap();
        let metadata = MetadataReader::new(&sql_path);
        let mz_reader: Tof2MzConverter = metadata.mz_converter;
        let tdf_sql_reader = SqlReader::open(&sql_path).unwrap();
        let precursor_reader: PrecursorReader = PrecursorReader::new(&sql_path);
        let spectrum_frame_index_reader =
            SpectrumFrameIndexReader::new(&tdf_sql_reader);
        Self {
            path: path_name.as_ref().to_path_buf(),
            precursor_reader,
            mz_reader,
            frame_reader,
            spectrum_frame_index_reader,
        }
    }

    pub fn read_single_raw_spectrum(&self, index: usize) -> RawSpectrum {
        let mut tof_indices: Vec<u32> = vec![];
        let mut intensities: Vec<u32> = vec![];
        for pasef_frame in self
            .spectrum_frame_index_reader
            .iterate_over_pasef_frames(index)
        {
            let frame_index: usize = pasef_frame.frame - 1;
            let frame = self.frame_reader.get(frame_index);
            if frame.intensities.len() == 0 {
                continue;
            }
            let scan_start: usize = pasef_frame.scan_start;
            let scan_end: usize = pasef_frame.scan_end;
            let offset_start: usize = frame.scan_offsets[scan_start] as usize;
            let offset_end: usize = frame.scan_offsets[scan_end] as usize;
            let tof_selection: &[u32] =
                &frame.tof_indices[offset_start..offset_end];
            let intensity_selection: &[u32] =
                &frame.intensities[offset_start..offset_end];
            tof_indices.extend(tof_selection);
            intensities.extend(intensity_selection);
        }
        let (raw_tof_indices, raw_intensities) = group_and_sum(
            tof_indices,
            intensities.iter().map(|x| *x as u64).collect(),
        );
        let raw_spectrum = RawSpectrum {
            tof_indices: raw_tof_indices,
            intensities: raw_intensities,
            index: index,
        };
        raw_spectrum
            .smooth(SMOOTHING_WINDOW)
            .centroid(CENTROIDING_WINDOW)
    }
}

impl SpectrumReaderTrait for TDFSpectrumReader {
    fn get(&self, index: usize) -> Spectrum {
        let raw_spectrum = self.read_single_raw_spectrum(index);
        let spectrum = raw_spectrum
            .finalize(self.precursor_reader.get(index), &self.mz_reader);
        spectrum
    }

    fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn calibrate(&mut self) {
        let hits: Vec<(f64, u32)> = (0..self.precursor_reader.len())
            .into_par_iter()
            .map(|index| {
                let spectrum = self.read_single_raw_spectrum(index);
                let precursor = self.precursor_reader.get(index);
                let precursor_mz: f64 = precursor.mz;
                let mut result: Vec<(f64, u32)> = vec![];
                for &tof_index in spectrum.tof_indices.iter() {
                    let mz = self.mz_reader.convert(tof_index);
                    if (mz - precursor_mz).abs() < CALIBRATION_TOLERANCE {
                        let hit = (precursor_mz, tof_index);
                        result.push(hit);
                    }
                }
                result
            })
            .reduce(Vec::new, |mut acc, mut vec| {
                acc.append(&mut vec); // Concatenate vectors
                acc
            });
        if hits.len() >= 2 {
            self.mz_reader = Tof2MzConverter::from_pairs(&hits);
        }
    }
}

#[derive(Debug)]
struct SpectrumFrameIndexReader {
    order: Vec<usize>,
    offsets: Vec<usize>,
    pasef_frames: Vec<SqlPasefFrameMsMs>,
}

impl SpectrumFrameIndexReader {
    fn new(tdf_sql_reader: &SqlReader) -> Self {
        let pasef_frames =
            SqlPasefFrameMsMs::from_sql_reader(&tdf_sql_reader).unwrap();
        let pasef_precursors =
            &pasef_frames.iter().map(|x| x.precursor).collect();
        let order: Vec<usize> = argsort(&pasef_precursors);
        let max_precursor = pasef_precursors.iter().max().unwrap();
        let mut offsets: Vec<usize> = Vec::with_capacity(max_precursor + 1);
        offsets.push(0);
        for (offset, &index) in order.iter().enumerate().take(order.len() - 1) {
            let second_index: usize = order[offset + 1];
            if pasef_precursors[index] != pasef_precursors[second_index] {
                offsets.push(offset + 1)
            }
        }
        offsets.push(order.len());
        Self {
            order,
            offsets,
            pasef_frames,
        }
    }

    fn iterate_over_pasef_frames(
        &self,
        index: usize,
    ) -> impl Iterator<Item = &SqlPasefFrameMsMs> {
        let start: usize = self.offsets[index];
        let end: usize = self.offsets[index + 1];
        self.order[start..end]
            .iter()
            .map(|&x| &self.pasef_frames[x])
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub(crate) struct RawSpectrum {
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u64>,
    pub index: usize,
}

impl RawSpectrum {
    pub fn smooth(mut self, window: u32) -> Self {
        let mut smooth_intensities: Vec<u64> = self.intensities.clone();
        for (current_index, current_tof) in self.tof_indices.iter().enumerate()
        {
            let current_intensity: u64 = self.intensities[current_index];
            for (_next_index, next_tof) in
                self.tof_indices[current_index + 1..].iter().enumerate()
            {
                let next_index: usize = _next_index + current_index + 1;
                let next_intensity: u64 = self.intensities[next_index];
                if (next_tof - current_tof) <= window {
                    smooth_intensities[current_index] += next_intensity;
                    smooth_intensities[next_index] += current_intensity;
                } else {
                    break;
                }
            }
        }
        self.intensities = smooth_intensities;
        self
    }

    pub fn centroid(mut self, window: u32) -> Self {
        let local_maxima: Vec<bool> = find_sparse_local_maxima_mask(
            &self.tof_indices,
            &self.intensities,
            window,
        );
        self.tof_indices = filter_with_mask(&self.tof_indices, &local_maxima);
        self.intensities = filter_with_mask(&self.intensities, &local_maxima);
        self
    }

    pub fn finalize(
        &self,
        precursor: Precursor,
        mz_reader: &Tof2MzConverter,
    ) -> Spectrum {
        let index = self.index;
        let spectrum: Spectrum = Spectrum {
            mz_values: self
                .tof_indices
                .iter()
                .map(|&x| mz_reader.convert(x))
                .collect(),
            intensities: self.intensities.iter().map(|x| *x as f64).collect(),
            precursor: precursor,
            index: index,
        };
        spectrum
    }
}
