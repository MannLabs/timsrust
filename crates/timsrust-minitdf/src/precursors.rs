use std::sync::Arc;

use serde::Deserialize;
use timsrust_core::io::formats::parquet::{ParquetError, ParquetReader};
use timsrust_core::utils::reader::Reader;
use timsrust_core::{
    Charge, Converter, FrameIndex, Im, Mz, Precursor, Rt, ScanIndex,
};

use crate::{MiniTDFError, MiniTDFPathError, timstof::MiniTDFPath};

#[derive(Debug)]
pub struct MiniTDFPrecursorReader {
    parquet_precursors: Vec<ParquetPrecursor>,
    im_converter: Arc<Scan2ImConverter>,
}

impl MiniTDFPrecursorReader {
    pub fn new(path: &MiniTDFPath) -> Result<Self, MiniTDFError> {
        let minitdf_path = path.ms2_parquet().clone();
        let parquet_precursors =
            ParquetReader::<ParquetPrecursor>::from(minitdf_path.as_ref())
                .map_err(MiniTDFPrecursorReaderError::from)?
                .read_all()
                .map_err(MiniTDFPrecursorReaderError::from)?;
        let reader = Self {
            parquet_precursors,
            im_converter: Arc::new(Scan2ImConverter()),
        };
        Ok(reader)
    }

    pub fn len(&self) -> usize {
        self.parquet_precursors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn im_converter(&self) -> &Arc<Scan2ImConverter> {
        &self.im_converter
    }
}

impl Reader<Precursor> for MiniTDFPrecursorReader {
    type Error = MiniTDFError;
    fn get(&self, index: usize) -> Result<Precursor, Self::Error> {
        let parquet_precursor = &self
            .parquet_precursors
            .get(index)
            .cloned()
            .ok_or(MiniTDFPrecursorReaderError::NoPrecursor)?;
        let precursor = Precursor::new(
            Mz::from(parquet_precursor.mz),
            Im::from(parquet_precursor.im),
            Rt::from(parquet_precursor.rt),
            self.im_converter.convert(Im::from(parquet_precursor.im)),
            Some(Charge::try_from(parquet_precursor.charge).unwrap()),
            Some(parquet_precursor.intensity),
            parquet_precursor.index,
            FrameIndex::try_from(parquet_precursor.frame_index).unwrap(),
        );
        Ok(precursor)
    }
}

#[derive(Debug, Clone)]
pub struct Scan2ImConverter();

impl Converter<ScanIndex, Im> for Scan2ImConverter {
    fn convert(&self, value: ScanIndex) -> Im {
        let bits = u32::from(value);
        Im::from(f32::from_bits(bits))
    }
}

impl Converter<Im, ScanIndex> for Scan2ImConverter {
    fn convert(&self, value: Im) -> ScanIndex {
        let bits = (f64::from(value) as f32).to_bits();
        ScanIndex::try_from(bits).expect("ScanIndex conversion out of bounds")
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MiniTDFPrecursorReaderError {
    #[error("{0}")]
    ParquetError(#[from] ParquetError),
    #[error("{0}")]
    MiniTDFPathError(#[from] MiniTDFPathError),
    #[error("No precursor found")]
    NoPrecursor,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub(crate) struct ParquetPrecursor {
    #[serde(rename = "MonoisotopicMz", default)]
    pub mz: f64,
    #[serde(rename = "RetentionTime", default)]
    pub rt: f64,
    #[serde(rename = "ooK0", default)]
    pub im: f64,
    #[serde(rename = "Charge", default)]
    pub charge: usize,
    #[serde(rename = "Intensity", default)]
    pub intensity: f64,
    #[serde(rename = "Id", default)]
    pub index: usize,
    #[serde(rename = "MS1ParentFrameId", default)]
    pub frame_index: usize,
    #[serde(rename = "BinaryOffset", default)]
    pub offset: u64,
    #[serde(rename = "CollisionEnergy", default)]
    pub collision_energy: f64,
}
