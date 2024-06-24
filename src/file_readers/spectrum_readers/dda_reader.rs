use std::path::Path;

use crate::{
    calibration::Tof2MzCalibrator,
    domain_converters::Tof2MzConverter,
    // file_readers::ReadableSpectra,
    io::readers::{
        file_readers::sql_reader::{
            pasef_frame_msms::SqlPasefFrameMsMs, ReadableSqlTable, SqlReader,
        },
        frame_reader::FrameReader,
        metadata_reader::MetadataReader,
        precursor_reader::PrecursorReader,
    },
    ms_data::{
        Frame, Precursor, RawProcessedSpectrumState, RawSpectrum,
        RawSpectrumProcessor, Spectrum,
    },
    utils::vec_utils::{argsort, group_and_sum},
};

use rayon::prelude::*;

// use self::precursors::PrecursorReader;

const SMOOTHING_WINDOW: u32 = 1;
const CENTROIDING_WINDOW: u32 = 1;

#[derive(Debug)]
pub struct DDASpectrumReader {
    pub path_name: String,
    precursor_reader: PrecursorReader,
    mz_reader: Tof2MzConverter,
    ms2_frames: Vec<Frame>,
    pub pasef_frames: Vec<SqlPasefFrameMsMs>,
    pub order: Vec<usize>,
    pub offsets: Vec<usize>,
}

impl DDASpectrumReader {
    pub fn new(path_name: String) -> Self {
        let frame_reader: FrameReader = FrameReader::new(&path_name);
        let metadata = MetadataReader::new(&path_name);
        let mz_reader: Tof2MzConverter = metadata.mz_converter;
        let tdf_sql_reader =
            SqlReader::open(Path::new(&path_name).join("analysis.tdf"))
                .unwrap();
        let pasef_frames =
            SqlPasefFrameMsMs::from_sql_reader(&tdf_sql_reader).unwrap();
        let ms2_frames: Vec<Frame> =
            frame_reader.parallel_filter(|x| x.msms_type != 0).collect();
        let precursor_reader: PrecursorReader =
            PrecursorReader::new(&path_name);
        let pasef_precursors =
            &pasef_frames.iter().map(|x| x.precursor).collect();
        let order: Vec<usize> = argsort(&pasef_precursors);
        let mut offsets: Vec<usize> =
            Vec::with_capacity(precursor_reader.len() + 1);
        offsets.push(0);
        for (offset, &index) in order.iter().enumerate().take(order.len() - 1) {
            let second_index: usize = order[offset + 1];
            if pasef_precursors[index] != pasef_precursors[second_index] {
                offsets.push(offset + 1)
            }
        }
        offsets.push(order.len());
        Self {
            path_name,
            precursor_reader,
            mz_reader,
            ms2_frames,
            pasef_frames,
            order,
            offsets,
        }
    }

    pub fn read_single_raw_spectrum(&self, index: usize) -> RawSpectrum {
        let start: usize = self.offsets[index];
        let end: usize = self.offsets[index + 1];
        let selection: &[usize] = &self.order[start..end];
        let mut tof_indices: Vec<u32> = vec![];
        let mut intensities: Vec<u32> = vec![];
        for &index in selection.iter() {
            let frame_index: usize = self.pasef_frames[index].frame - 1;
            // TODO OPTIMIZE!!!!!
            let frame: &Frame = &self
                .ms2_frames
                .iter()
                .find(|&x| x.index == frame_index + 1)
                .unwrap();
            if frame.intensities.len() == 0 {
                continue;
            }
            let scan_start: usize = self.pasef_frames[index].scan_start;
            let scan_end: usize = self.pasef_frames[index].scan_end;
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
            processed_state: RawProcessedSpectrumState::Profile,
            index: index,
        };
        let spectrum_processer = RawSpectrumProcessor { raw_spectrum };
        spectrum_processer
            .smooth(SMOOTHING_WINDOW)
            .centroid(CENTROIDING_WINDOW)
            .raw_spectrum
    }

    pub fn process_single_raw_spectrum(
        &self,
        raw_spectrum: RawSpectrum,
        mz_reader: &Tof2MzConverter,
    ) -> Spectrum {
        let index: usize = raw_spectrum.index as usize;
        let spectrum_processer = RawSpectrumProcessor { raw_spectrum };
        let spectrum = spectrum_processer
            .finalize(self.precursor_reader.get(index), mz_reader);
        spectrum
    }
}

impl DDASpectrumReader {
    pub fn read_single_spectrum(&self, index: usize) -> Spectrum {
        let raw_spectrum = self.read_single_raw_spectrum(index);
        self.process_single_raw_spectrum(raw_spectrum, &self.mz_reader)
    }

    pub fn read_all_spectra(&self) -> Vec<Spectrum> {
        let raw_spectra: Vec<RawSpectrum> = (0..self.precursor_reader.len())
            .into_par_iter()
            .map(|index| self.read_single_raw_spectrum(index))
            .collect();
        let precursors: Vec<Precursor> = (0..self.precursor_reader.len())
            .map(|index| self.precursor_reader.get(index))
            .collect();
        let hits = Tof2MzCalibrator::find_unfragmented_precursors(
            &raw_spectra,
            &self.mz_reader,
            &precursors,
            0.1,
        );
        let temp_mz_reader: Tof2MzConverter;
        let mz_reader: &Tof2MzConverter;
        if hits.len() >= 2 {
            temp_mz_reader = Tof2MzConverter::from_pairs(&hits);
            mz_reader = &temp_mz_reader;
        } else {
            mz_reader = &self.mz_reader
        }
        let spectra: Vec<Spectrum> = raw_spectra
            .into_par_iter()
            .map(|spectrum| {
                self.process_single_raw_spectrum(spectrum, &mz_reader)
            })
            .collect();
        spectra
    }
}
