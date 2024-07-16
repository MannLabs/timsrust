use std::path::{Path, PathBuf};

use crate::{
    io::readers::{
        file_readers::{
            parquet_reader::{
                precursors::ParquetPrecursor, ReadableParquetTable,
            },
            tdf_blob_reader::{
                IndexedTdfBlobReader, TdfBlobError, TdfBlobReaderError,
            },
        },
        PrecursorReader,
    },
    ms_data::Spectrum,
    utils::find_extension,
};

use super::SpectrumReaderTrait;

#[derive(Debug)]
pub struct MiniTDFSpectrumReader {
    path: PathBuf,
    precursor_reader: PrecursorReader,
    blob_reader: IndexedTdfBlobReader,
    collision_energies: Vec<f64>,
}

impl MiniTDFSpectrumReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let parquet_file_name =
            find_extension(&path, "ms2spectrum.parquet").unwrap();
        let precursor_reader = PrecursorReader::new(&parquet_file_name);
        let offsets = ParquetPrecursor::from_parquet_file(&parquet_file_name)
            .unwrap()
            .iter()
            .map(|x| x.offset as usize)
            .collect();
        let collision_energies =
            ParquetPrecursor::from_parquet_file(&parquet_file_name)
                .unwrap()
                .iter()
                .map(|x| x.collision_energy)
                .collect();
        let bin_file_name = find_extension(&path, "bin").unwrap();
        let blob_reader =
            IndexedTdfBlobReader::new(&bin_file_name, offsets).unwrap();
        Self {
            path: path.as_ref().to_path_buf(),
            precursor_reader,
            blob_reader,
            collision_energies,
        }
    }
}

impl SpectrumReaderTrait for MiniTDFSpectrumReader {
    fn get(&self, index: usize) -> Spectrum {
        let mut spectrum = Spectrum::default();
        spectrum.index = index;
        let blob = self.blob_reader.get_blob(index).unwrap();
        if !blob.is_empty() {
            let size: usize = blob.len();
            let spectrum_data: Vec<u32> =
                (0..size).map(|i| blob.get(i).unwrap()).collect();
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
        let precursor = self.precursor_reader.get(index);
        spectrum.precursor = Some(precursor);
        spectrum.index = precursor.index;
        spectrum.collision_energy = self.collision_energies[index];
        spectrum.isolation_mz = precursor.mz; //FIX?
        spectrum.isolation_width = if precursor.mz <= 700.0 {
            2.0
        } else if precursor.mz >= 800.0 {
            3.0
        } else {
            2.0 + (precursor.mz - 700.0) / 100.0
        }; //FIX?
        spectrum
    }

    fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn calibrate(&mut self) {}
}
