use crate::{
    io::readers::{
        file_readers::{
            parquet_reader::{
                precursors::ParquetPrecursor, ParquetReaderError,
                ReadableParquetTable,
            },
            tdf_blob_reader::{
                IndexedTdfBlobReader, IndexedTdfBlobReaderError,
            },
        },
        PrecursorReader, PrecursorReaderError,
    },
    ms_data::Spectrum,
    readers::TimsTofPathLike,
};

use super::{SpectrumReaderError, SpectrumReaderTrait};

#[derive(Debug)]
pub struct MiniTDFSpectrumReader {
    precursor_reader: PrecursorReader,
    blob_reader: IndexedTdfBlobReader,
    collision_energies: Vec<f64>,
}

impl MiniTDFSpectrumReader {
    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Self, MiniTDFSpectrumReaderError> {
        let precursor_reader =
            PrecursorReader::build().with_path(&path).finalize()?;
        let offsets = ParquetPrecursor::from_parquet_file(&path)?
            .iter()
            .map(|x| x.offset as usize)
            .collect();
        let collision_energies = ParquetPrecursor::from_parquet_file(&path)?
            .iter()
            .map(|x| x.collision_energy)
            .collect();
        let blob_reader = IndexedTdfBlobReader::new(&path, offsets)?;
        let reader = Self {
            precursor_reader,
            blob_reader,
            collision_energies,
        };
        Ok(reader)
    }

    fn _get(
        &self,
        index: usize,
    ) -> Result<Spectrum, MiniTDFSpectrumReaderError> {
        let mut spectrum = Spectrum::default();
        spectrum.index = index;
        let blob = self.blob_reader.get(index)?;
        if !blob.is_empty() {
            let spectrum_data: Vec<u32> = blob.get_all();
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
        let precursor = self
            .precursor_reader
            .get(index)
            .ok_or(MiniTDFSpectrumReaderError::NoPrecursor)?;
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
        Ok(spectrum)
    }
}

impl SpectrumReaderTrait for MiniTDFSpectrumReader {
    fn get(&self, index: usize) -> Result<Spectrum, SpectrumReaderError> {
        Ok(self._get(index)?)
    }

    fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    fn calibrate(&mut self) {}
}

#[derive(Debug, thiserror::Error)]
pub enum MiniTDFSpectrumReaderError {
    #[error("{0}")]
    PrecursorReaderError(#[from] PrecursorReaderError),
    #[error("{0}")]
    ParquetReaderError(#[from] ParquetReaderError),
    #[error("{0}")]
    IndexedTdfBlobReaderError(#[from] IndexedTdfBlobReaderError),
    #[error("{0}")]
    FileNotFound(String),
    #[error("No precursor")]
    NoPrecursor,
    #[error("BlobError")]
    BlobError,
}
