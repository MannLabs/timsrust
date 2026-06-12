use std::collections::HashMap;

use serde::Deserialize;
use timsrust_core::{
    IsolationWindow, Spectrum,
    io::formats::sql::{SqlError, SqlReader},
};

use crate::Tof2MzConverter;
use crate::timstof::TSFPathError;
use crate::{
    blobs::{TsfBlobReader, TsfBlobReaderError},
    timstof::TSFPathLike,
};

#[derive(Deserialize)]
struct KvRow {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
}

#[derive(Deserialize)]
struct SqlFrame {
    #[serde(rename = "Id")]
    id: i64,
    #[serde(rename = "NumPeaks")]
    num_peaks: i64,
    #[serde(rename = "Time")]
    time: f64,
    #[serde(rename = "TimsId")]
    tims_id: i64,
}

#[derive(Debug)]
pub struct TSFSpectrumReader {
    frames: Vec<TsfFrame>,
    blob_reader: TsfBlobReader,
    mz_converter: Tof2MzConverter,
}

impl TSFSpectrumReader {
    pub fn new(path: impl TSFPathLike) -> Result<Self, TSFSpectrumReaderError> {
        let blob_reader = TsfBlobReader::new(&path)?;
        let timstof_path = path.to_timstof_path()?;
        let mz_converter = Tof2MzConverter::new(timstof_path.as_ref());
        let reader = SqlReader::from(timstof_path.tsf().as_ref())?;
        let metadata: HashMap<String, String> = reader
            .from_table::<KvRow>("GlobalMetadata")?
            .read_all()?
            .into_iter()
            .map(|r| (r.key, r.value))
            .collect();
        let has_line_spectra = metadata
            .get("HasLineSpectra")
            .map(|v| v.trim() == "1")
            .unwrap_or(false);
        if !has_line_spectra {
            return Err(TSFSpectrumReaderError::UnsupportedDataset);
        }
        let frames: Vec<TsfFrame> = reader
            .from_table::<SqlFrame>("Frames")?
            .read_all()?
            .into_iter()
            .map(|f| TsfFrame {
                _frame_id: f.id as usize,
                num_peaks: f.num_peaks as usize,
                _rt_seconds: f.time,
                offset: f.tims_id as usize,
            })
            .collect();
        Ok(Self {
            frames,
            blob_reader,
            mz_converter,
        })
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn mz_converter(&self) -> &Tof2MzConverter {
        &self.mz_converter
    }
}

impl timsrust_core::utils::reader::IndexedReader<Spectrum>
    for TSFSpectrumReader
{
    type Iter = std::ops::Range<usize>;
    fn iter(&self) -> Self::Iter {
        0..self.len()
    }
}

impl timsrust_core::utils::reader::Reader<Spectrum> for TSFSpectrumReader {
    type Error = TSFSpectrumReaderError;
    fn get(&self, index: usize) -> Result<Spectrum, Self::Error> {
        let frame = self
            .frames
            .get(index)
            .ok_or(TSFSpectrumReaderError::IndexOutOfBounds)?;

        let chunk =
            self.blob_reader.read_chunk(frame.offset, frame.num_peaks)?;
        let isolation_window = IsolationWindow::default();
        let spectrum = Spectrum::new(
            chunk.intensities,
            index,
            None,
            chunk.tof_indices,
            isolation_window,
        );
        Ok(spectrum)
    }
}

#[derive(Debug)]
struct TsfFrame {
    _frame_id: usize,
    offset: usize,
    num_peaks: usize,
    _rt_seconds: f64,
}

// Minimal frame data extracted from the TSF 'Frames' table.

#[derive(Debug, thiserror::Error)]
pub enum TSFSpectrumReaderError {
    // #[error("{0}")]
    // SqlReaderError(#[from] SqlReaderError),
    // #[error("{0}")]
    // MetadataReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    Sql(#[from] SqlError),
    #[error("{0}")]
    TSFPathError(#[from] TSFPathError),
    #[allow(private_interfaces)]
    #[error("{0}")]
    TsfBlobReaderError(#[from] TsfBlobReaderError),
    #[error("Spectrum index out of bounds")]
    IndexOutOfBounds,
    #[error("TSF dataset does not contain centroided line spectra")]
    UnsupportedDataset,
}
