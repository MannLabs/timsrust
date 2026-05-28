use serde::{Deserialize, Serialize};
use timsrust_core::io::formats::parquet::ParquetReader;
use timsrust_core::utils::enumerated_error;
use timsrust_core::{ConvertibleTo, Mz, Spectrum};

use crate::Tof2MzConverter;
use crate::precursor_reader::{
    ParquetPrecursorReader, ParquetPrecursorReaderError,
};

#[derive(Debug)]
pub struct ParquetSpectrumReader {
    precursor_reader: ParquetPrecursorReader,
    peak_reader: ParquetReader<CoordinatePeak>,
    // fragments: Vec<CoordinatePeak>,
    mz_converter: Tof2MzConverter,
}

impl ParquetSpectrumReader {
    pub fn new(ms1_path: impl AsRef<str>, ms2_path: impl AsRef<str>) -> Self {
        let precursor_reader = ParquetPrecursorReader::new(ms1_path);
        // let fragments =
        //     CoordinatePeak::deserialize_vec_from_uri(ms2_path.as_ref())
        //         .unwrap();
        let peak_reader =
            ParquetReader::<CoordinatePeak>::from(ms2_path.as_ref()).unwrap();
        // let fragments = reader.get(0..reader.len()).unwrap();
        // dbg!(fragments.len());
        Self {
            precursor_reader,
            // fragments,
            peak_reader,
            mz_converter: Tof2MzConverter(),
        }
    }

    pub fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn precursor_reader(&self) -> &ParquetPrecursorReader {
        &self.precursor_reader
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct CoordinatePeak {
    // pub frame: u32,
    // pub scan: u32,
    // pub tof: u32,
    pub apex_intensity: u64,
    // pub rt: f64,
    // pub im: f64,
    pub mz: f64,
}

// impl_parquet_scheme!(
//     CoordinatePeak,
//     [
//         // (frame, arrow::datatypes::UInt32Type, false),
//         // (scan, arrow::datatypes::UInt32Type, false),
//         // (tof, arrow::datatypes::UInt32Type, false),
//         (apex_intensity, arrow::datatypes::UInt64Type, false),
//         // (rt, arrow::datatypes::Float64Type, false),
//         // (im, arrow::datatypes::Float64Type, false),
//         (mz, arrow::datatypes::Float64Type, false),
//     ]
// );

// #[derive(Clone, Debug, PartialEq, Default)]
// pub struct CoordinatePeak {
//     // pub frame: u32,
//     // pub scan: u32,
//     // pub tof: u32,
//     // pub rt: f64,
//     // pub im: f64,
//     pub mz: f64,
//     pub apex_intensity: u64,
// }

impl timsrust_core::utils::reader::IndexedReader<Spectrum>
    for ParquetSpectrumReader
{
    type Iter = std::ops::Range<usize>;
    fn iter(&self) -> Self::Iter {
        0..self.len()
    }
}

impl timsrust_core::utils::reader::Reader<Spectrum> for ParquetSpectrumReader {
    type Error = ParquetSpectrumReaderError;

    fn get(&self, index: usize) -> Result<Spectrum, Self::Error> {
        let precursor: crate::precursor_reader::Precursor =
            self.precursor_reader.get(index)?;
        let isolation_window = timsrust_core::IsolationWindow::new_from_center(
            timsrust_core::Mz::from(precursor.isolation_mz),
            timsrust_core::Mz::from(precursor.isolation_width),
            precursor.ce,
        );
        let start = precursor.start as usize;
        let end = precursor.end as usize;
        let mut fragments = self
            .peak_reader
            .read_range(start..end)
            .unwrap()
            .iter()
            .map(|frag| {
                let mz = Mz::from(frag.mz);
                let tof = mz.convert(&self.mz_converter);
                (tof, frag.apex_intensity as f64)
            })
            .collect::<Vec<_>>();
        fragments.sort_by_key(|(tof, _)| *tof);
        let (tof_indices, intensities) = fragments.into_iter().unzip();
        let spectrum = Spectrum::new(
            intensities,
            index,
            Some(precursor.into()),
            tof_indices,
            isolation_window,
        );
        Ok(spectrum)
    }
}

enumerated_error!(
    pub ParquetSpectrumReaderError,
    ParquetPrecursorReader(ParquetPrecursorReaderError)
);
