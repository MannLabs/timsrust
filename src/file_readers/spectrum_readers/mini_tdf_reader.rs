use {
    crate::{
        file_readers::{
            common::{
                ms_data_blobs::{BinFileReader, ReadableFromBinFile},
                parquet_reader::read_parquet_precursors,
            },
            ReadableSpectra,
        },
        precursors::QuadrupoleEvent,
        Precursor, Spectrum,
    },
    rayon::prelude::*,
    std::path::PathBuf,
};

#[derive(Debug, Default, Clone)]
pub struct MiniTDFReader {
    pub path_name: String,
    parquet_file_name: String,
    precursors: Vec<Precursor>,
    offsets: Vec<u64>,
    frame_reader: BinFileReader,
}

impl MiniTDFReader {
    pub fn new(path_name: String) -> Self {
        let mut reader: MiniTDFReader = Self::default();
        reader.path_name = path_name;
        reader.read_parquet_file_name();
        reader.read_precursors();
        reader.set_spectrum_reader();
        reader
    }

    fn read_parquet_file_name(&mut self) {
        let mut path: PathBuf = PathBuf::from(&self.path_name);
        path.push("converter.MS2Spectra.ms2.parquet");
        self.parquet_file_name = path.to_string_lossy().into_owned();
    }

    fn read_precursors(&mut self) {
        (self.precursors, self.offsets) =
            read_parquet_precursors(&self.parquet_file_name);
    }
    fn set_spectrum_reader(&mut self) {
        let mut path: PathBuf = PathBuf::from(&self.path_name);
        path.push("converter.ms2.bin");
        let file_name: String = path.to_string_lossy().into_owned();
        self.frame_reader =
            BinFileReader::new(String::from(&file_name), self.offsets.clone());
    }
}

impl ReadableSpectra for MiniTDFReader {
    fn read_single_spectrum(&self, index: usize) -> Spectrum {
        let mut spectrum: Spectrum =
            Spectrum::read_from_file(&self.frame_reader, index);
        spectrum.precursor = QuadrupoleEvent::Precursor(self.precursors[index]);
        spectrum
    }

    fn read_all_spectra(&self) -> Vec<Spectrum> {
        let size: usize = self.offsets.len();
        let mut spectra: Vec<Spectrum> = (0..size)
            .into_par_iter()
            .map(|index| self.read_single_spectrum(index))
            .collect();
        spectra.sort_by(|a, b| {
            let x = b.precursor.unwrap_as_precursor().index as f64;
            let y = a.precursor.unwrap_as_precursor().index as f64;
            y.total_cmp(&x)
        });
        spectra
    }
}
