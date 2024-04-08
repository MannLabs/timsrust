mod precursors;

use crate::{
    calibration::Tof2MzCalibrator,
    file_readers::{
        frame_readers::{tdf_reader::TDFReader, ReadableFrames},
        ReadableSpectra,
    },
    ms_data::spectra::{self, RawSpectrum, RawSpectrumProcessor},
    utils::vec_utils::group_and_sum,
    Frame, Spectrum, Tof2MzConverter,
};

use rayon::prelude::*;

use self::precursors::PrecursorReader;

const SMOOTHING_WINDOW: u32 = 1;
const CENTROIDING_WINDOW: u32 = 1;

#[derive(Debug)]
pub struct DDASpectrumReader {
    pub path_name: String,
    precursor_reader: PrecursorReader,
    mz_reader: Tof2MzConverter,
    ms2_frames: Vec<Frame>,
}

impl DDASpectrumReader {
    pub fn new(path_name: String) -> Self {
        let tdf_reader: TDFReader = TDFReader::new(&path_name.to_string());
        let mz_reader: Tof2MzConverter = tdf_reader.mz_converter;
        let ms2_frames: Vec<Frame> = tdf_reader.read_all_ms2_frames();
        let precursor_reader: PrecursorReader =
            PrecursorReader::new(&tdf_reader);
        Self {
            path_name,
            precursor_reader,
            mz_reader,
            ms2_frames,
        }
    }

    pub fn read_single_raw_spectrum(&self, index: usize) -> RawSpectrum {
        let start: usize = self.precursor_reader.offsets[index];
        let end: usize = self.precursor_reader.offsets[index + 1];
        let selection: &[usize] = &self.precursor_reader.order[start..end];
        let mut tof_indices: Vec<u32> = vec![];
        let mut intensities: Vec<u32> = vec![];
        for &index in selection.iter() {
            let frame: usize =
                self.precursor_reader.pasef_frames.frame[index] - 1;
            if self.ms2_frames[frame].intensities.len() == 0 {
                continue;
            }
            let scan_start: usize =
                self.precursor_reader.pasef_frames.scan_start[index];
            let scan_end: usize =
                self.precursor_reader.pasef_frames.scan_end[index];
            let offset_start: usize =
                self.ms2_frames[frame].scan_offsets[scan_start] as usize;
            let offset_end: usize =
                self.ms2_frames[frame].scan_offsets[scan_end] as usize;
            let tof_selection: &[u32] =
                &self.ms2_frames[frame].tof_indices[offset_start..offset_end];
            let intensity_selection: &[u32] =
                &self.ms2_frames[frame].intensities[offset_start..offset_end];
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
            processed_state: spectra::RawProcessedSpectrumState::Profile,
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
            .finalize(self.precursor_reader.precursors[index], mz_reader);
        spectrum
    }
}

impl ReadableSpectra for DDASpectrumReader {
    fn read_single_spectrum(&self, index: usize) -> Spectrum {
        let raw_spectrum = self.read_single_raw_spectrum(index);
        self.process_single_raw_spectrum(raw_spectrum, &self.mz_reader)
    }

    fn read_all_spectra(&self) -> Vec<Spectrum> {
        let raw_spectra: Vec<RawSpectrum> = (0..self.precursor_reader.count)
            .into_par_iter()
            .map(|index| self.read_single_raw_spectrum(index))
            .collect();
        let hits = Tof2MzCalibrator::find_unfragmented_precursors(
            &raw_spectra,
            &self.mz_reader,
            &self.precursor_reader.precursors,
            0.1,
        );
        let mz_reader: Tof2MzConverter;
        if hits.len() >= 2 {
            mz_reader = Tof2MzConverter::from_pairs(&hits);
        } else {
            mz_reader = self.mz_reader
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
