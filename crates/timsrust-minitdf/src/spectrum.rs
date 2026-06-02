use timsrust_core::io::formats::parquet::{ParquetError, ParquetReader};
use timsrust_core::{Converter, Mz, Spectrum};

use crate::{
    MiniTDFError,
    precursors::{
        MiniTDFPrecursorReader, MiniTDFPrecursorReaderError, ParquetPrecursor,
    },
    tdf_blob::{IndexedTdfBlobReader, IndexedTdfBlobReaderError},
    timstof::{MiniTDFPath, MiniTDFPathError},
};

#[derive(Debug)]
pub struct MiniTDFSpectrumReader {
    precursor_reader: MiniTDFPrecursorReader,
    blob_reader: IndexedTdfBlobReader,
    collision_energies: Vec<f64>,
    mz_converter: timsrust_core::BitConverter,
}

impl MiniTDFSpectrumReader {
    pub fn new(path: &MiniTDFPath) -> Result<Self, MiniTDFError> {
        let precursor_reader = MiniTDFPrecursorReader::new(path)?;
        let minitdf_path = path.ms2_parquet().clone();
        let all_precursors =
            ParquetReader::<ParquetPrecursor>::from(minitdf_path.as_str())
                .map_err(MiniTDFSpectrumReaderError::from)?
                .read_all()
                .map_err(MiniTDFSpectrumReaderError::from)?;
        let offsets =
            all_precursors.iter().map(|x| x.offset as usize).collect();
        let collision_energies =
            all_precursors.iter().map(|x| x.collision_energy).collect();
        let blob_reader = IndexedTdfBlobReader::new(path, offsets)
            .map_err(MiniTDFSpectrumReaderError::from)?;
        let reader = Self {
            precursor_reader,
            blob_reader,
            collision_energies,
            mz_converter: timsrust_core::BitConverter(),
        };
        Ok(reader)
    }

    pub fn len(&self) -> usize {
        self.collision_energies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl timsrust_core::utils::reader::Reader<Spectrum> for MiniTDFSpectrumReader {
    type Error = MiniTDFError;
    fn get(&self, index: usize) -> Result<Spectrum, Self::Error> {
        // let mut spectrum = timsrust_core::Spectrum {
        //     index,
        //     ..Default::default()
        // };
        let blob = self
            .blob_reader
            .get(index)
            .map_err(MiniTDFSpectrumReaderError::from)?;
        let (intensities, tof_indices) = if !blob.is_empty() {
            let spectrum_data: Vec<u32> = blob.get_all();
            let scan_count: usize = blob.len() / 3;
            let tof_indices_bytes: &[u32] = &spectrum_data[..scan_count * 2];
            let intensities_bytes: &[u32] = &spectrum_data[scan_count * 2..];
            let mz_values: &[f64] =
                bytemuck::cast_slice::<u32, f64>(tof_indices_bytes);
            let intensity_values: &[f32] =
                bytemuck::cast_slice::<u32, f32>(intensities_bytes);
            let intensities =
                intensity_values.iter().map(|&x| x as f64).collect();
            // spectrum.mz_values =
            //     mz_values.iter().map(|&x| Mz::new(x as f32)).collect();
            let tof_indices = mz_values
                .iter()
                .map(|&x| self.mz_converter.convert(Mz::from(x)))
                .collect();
            (intensities, tof_indices)
        } else {
            (vec![], vec![])
        };
        let precursor: timsrust_core::Precursor =
            self.precursor_reader.get(index)?;
        // spectrum.precursor = Some(precursor.clone());
        // spectrum.index = precursor.index;
        let precursor_mz = f64::from(precursor.mz());
        let collision_energy = self.collision_energies[index];
        let isolation_mz = Mz::from(precursor_mz); //FIX?
        let isolation_width = if precursor_mz <= 700.0 {
            2.0
        } else if precursor_mz >= 800.0 {
            3.0
        } else {
            2.0 + (precursor_mz - 700.0) / 100.0
        }; //FIX?
        let isolation_window = timsrust_core::IsolationWindow::new_from_center(
            isolation_mz,
            Mz::from(isolation_width),
            collision_energy,
        );
        let spectrum = Spectrum::new(
            intensities,
            index,
            Some(precursor),
            tof_indices,
            isolation_window,
        );
        Ok(spectrum)
    }
}

impl timsrust_core::utils::reader::IndexedReader<Spectrum>
    for MiniTDFSpectrumReader
{
    type Iter = std::ops::Range<usize>;
    fn iter(&self) -> Self::Iter {
        0..self.len()
    }
}

// timsrust_core::impl_par_iter!(MiniTDFSpectrumReader, Spectrum);

#[derive(Debug, thiserror::Error)]
pub(crate) enum MiniTDFSpectrumReaderError {
    #[error("{0}")]
    PrecursorReader(#[from] MiniTDFPrecursorReaderError),
    #[error("{0}")]
    Parquet(#[from] ParquetError),
    #[error("{0}")]
    IndexedTdfBlobReader(#[from] IndexedTdfBlobReaderError),
    #[error("{0}")]
    MiniTDFPath(#[from] MiniTDFPathError),
}
