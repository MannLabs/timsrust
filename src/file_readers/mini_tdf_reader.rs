use crate::{
    file_readers::FileFormatError,
    io::readers::{
        file_readers::{
            parquet_reader::{
                precursors::ParquetPrecursor, ReadableParquetTable,
            },
            tdf_blob_reader::{IndexedTdfBlobReader, TdfBlob, TdfBlobError},
        },
        PrecursorReader,
    },
};
use std::fs;
use {crate::ms_data::Spectrum, rayon::prelude::*, std::path::PathBuf};

#[derive(Debug)]
pub struct MiniTDFReader {
    pub path_name: String,
    precursor_reader: PrecursorReader,
    blob_reader: IndexedTdfBlobReader,
}

fn find_ms2spectrum_file(
    ms2_dir_path: &str,
    extension: String,
) -> Result<String, FileFormatError> {
    let files = fs::read_dir(ms2_dir_path).unwrap();
    for file in files {
        let filename = file
            .unwrap()
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        if filename
            .ends_with(std::format!("ms2spectrum.{}", extension).as_str())
        {
            return Ok(filename);
        }
    }
    let err = match extension.as_str() {
        "parquet" => FileFormatError::MetadataFilesAreMissing,
        "bin" => FileFormatError::BinaryFilesAreMissing,
        _ => FileFormatError::BinaryFilesAreMissing,
    };
    println!(
        "{}",
        format!(
            "No '*.ms2spectrum.{}' file found in '{}'",
            extension, ms2_dir_path
        )
    );
    return Err(err);
}

impl MiniTDFReader {
    pub fn new(path_name: String) -> Self {
        let parquet_file_name = Self::read_parquet_file_name(&path_name);
        let precursor_reader = PrecursorReader::new(&parquet_file_name);
        let offsets = Self::get_offsets(&parquet_file_name);
        let blob_reader =
            Self::get_spectrum_reader(&path_name, offsets).unwrap();
        Self {
            path_name,
            precursor_reader,
            blob_reader,
        }
    }

    fn read_parquet_file_name(path_name: &String) -> String {
        let mut path: PathBuf = PathBuf::from(&path_name);
        let ms2_parquet_file =
            find_ms2spectrum_file(&path_name, "parquet".to_owned()).unwrap();
        path.push(ms2_parquet_file);
        path.to_string_lossy().into_owned()
    }

    fn get_offsets(parquet_file_name: &String) -> Vec<usize> {
        let parquet_precursors =
            ParquetPrecursor::from_parquet_file(&parquet_file_name).unwrap();
        parquet_precursors
            .iter()
            .map(|x| x.offset as usize)
            .collect()
    }

    fn get_spectrum_reader(
        path_name: &String,
        offsets: Vec<usize>,
    ) -> Result<IndexedTdfBlobReader, TdfBlobError> {
        let mut path: PathBuf = PathBuf::from(&path_name);
        let ms2_bin_file =
            find_ms2spectrum_file(&path_name, "bin".to_owned()).unwrap();
        path.push(ms2_bin_file);
        let file_name: String = path.to_string_lossy().into_owned();
        IndexedTdfBlobReader::new(String::from(&file_name), offsets)
    }

    pub fn read_single_spectrum(&self, index: usize) -> Spectrum {
        let mut spectrum: Spectrum =
            Self::create_from_tdf_blob_reader(&self, index);
        let precursor = self.precursor_reader.get(index);
        spectrum.precursor = precursor;
        spectrum.index = precursor.index;
        spectrum
    }

    pub fn read_all_spectra(&self) -> Vec<Spectrum> {
        let size: usize = self.precursor_reader.len();
        let mut spectra: Vec<Spectrum> = (0..size)
            .into_par_iter()
            .map(|index| self.read_single_spectrum(index))
            .collect();
        spectra.sort_by(|a, b| {
            let x = b.precursor.index as f64;
            let y = a.precursor.index as f64;
            y.total_cmp(&x)
        });
        spectra
    }

    fn update_from_tdf_blob(spectrum: &mut Spectrum, blob: TdfBlob) {
        let size: usize = blob.len();
        let spectrum_data: Vec<u32> = (0..size).map(|i| blob.get(i)).collect();
        let scan_count: usize = blob.len() / 3;
        let tof_indices_bytes: &[u32] =
            &spectrum_data[..scan_count as usize * 2];
        let intensities_bytes: &[u32] =
            &spectrum_data[scan_count as usize * 2..];
        let mz_values: &[f64] =
            bytemuck::cast_slice::<u32, f64>(tof_indices_bytes);
        let intensity_values: &[f32] =
            bytemuck::cast_slice::<u32, f32>(intensities_bytes);
        spectrum.intensities =
            intensity_values.iter().map(|&x| x as f64).collect();
        spectrum.mz_values = mz_values.to_vec();
    }

    fn create_from_tdf_blob_reader(&self, index: usize) -> Spectrum {
        let mut spectrum = Spectrum::default();
        spectrum.index = index;
        let blob = self.blob_reader.get_blob(index).unwrap();
        if !blob.is_empty() {
            Self::update_from_tdf_blob(&mut spectrum, blob)
        }
        spectrum
    }
}
