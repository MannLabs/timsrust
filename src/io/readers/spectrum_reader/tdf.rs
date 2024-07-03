mod dda;
mod raw_spectra;

use dda::RawSpectrumReader;
use raw_spectra::RawSpectrum;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};

use crate::{
    domain_converters::{ConvertableDomain, Tof2MzConverter},
    io::readers::{
        file_readers::sql_reader::SqlReader, FrameReader, MetadataReader,
        PrecursorReader,
    },
    ms_data::{AcquisitionType, Spectrum},
    utils::find_extension,
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
    spectrum_frame_index_reader: RawSpectrumReader,
}

impl TDFSpectrumReader {
    pub fn new(path_name: impl AsRef<Path>) -> Self {
        let frame_reader: FrameReader = FrameReader::new(&path_name);
        let sql_path = find_extension(&path_name, "analysis.tdf").unwrap();
        let metadata = MetadataReader::new(&sql_path);
        let mz_reader: Tof2MzConverter = metadata.mz_converter;
        let tdf_sql_reader = SqlReader::open(&sql_path).unwrap();
        let precursor_reader;
        let spectrum_frame_index_reader;
        if frame_reader.get_acquisition() == AcquisitionType::DDAPASEF {
            precursor_reader = PrecursorReader::new(&sql_path);
            spectrum_frame_index_reader =
                RawSpectrumReader::new(&tdf_sql_reader, frame_reader);
        } else {
            // TODO parse diaPASEF
            panic!("Not DDA")
        }
        Self {
            path: path_name.as_ref().to_path_buf(),
            precursor_reader,
            mz_reader,
            spectrum_frame_index_reader,
        }
    }

    pub fn read_single_raw_spectrum(&self, index: usize) -> RawSpectrum {
        let raw_spectrum =
            self.spectrum_frame_index_reader.get_raw_spectrum(index);
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
