use std::io::{self, Cursor};
use timsrust_core::{
    TofIndex,
    io::formats::binary::{BinaryError, BinaryReader},
};
use zstd::decode_all;

use crate::timstof::{TSFPathError, TSFPathLike};

const HEADER_BYTES: usize = 8;

#[derive(Debug)]
pub(crate) struct TsfBlobReader {
    binary_file: BinaryReader,
}

impl TsfBlobReader {
    pub(crate) fn new(
        path: impl TSFPathLike,
    ) -> Result<Self, TsfBlobReaderError> {
        let path = path.to_timstof_path()?;
        let bin_path = path.tsf_bin();
        let binary_file = BinaryReader::from(bin_path.as_str())?;
        let reader = Self { binary_file };
        Ok(reader)
    }

    pub(crate) fn read_chunk(
        &self,
        offset: usize,
        num_peaks: usize,
    ) -> Result<TsfSpectrumChunk, TsfBlobReaderError> {
        let header = self.read_header(offset)?;
        let compressed_start = offset + HEADER_BYTES;
        let compressed_end = compressed_start + header.compressed_len;
        let compressed = self
            .binary_file
            .read_range(compressed_start..compressed_end)?;

        if compressed.is_empty() {
            return Ok(TsfSpectrumChunk {
                tof_indices: Vec::new(),
                intensities: Vec::new(),
            });
        }

        let decompressed = decode_all(Cursor::new(compressed))
            .map_err(TsfBlobReaderError::Decompression)?;
        // check that the number of peaks matches with the decompressed data length.
        // Each peak uses 12 bytes
        let expected = num_peaks
            .checked_mul(12)
            .ok_or(TsfBlobReaderError::Overflow)?;
        if decompressed.len() < expected {
            return Err(TsfBlobReaderError::UnexpectedLength {
                expected,
                actual: decompressed.len(),
            });
        }
        let (tof_indices_bytes, intensity_bytes) =
            decompressed.split_at(num_peaks * 8);
        let tof_indices = tof_indices_bytes
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
            .map(|tof_index| {
                TofIndex::try_from(tof_index as u32)
                    .expect("TofIndex conversion out of bounds")
            })
            .collect();
        let intensities = intensity_bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .map(|value| value as f64)
            .collect();
        Ok(TsfSpectrumChunk {
            tof_indices,
            intensities,
        })
    }

    fn read_header(
        &self,
        offset: usize,
    ) -> Result<TsfChunkHeader, TsfBlobReaderError> {
        let header_bytes =
            self.binary_file.read_range(offset..offset + HEADER_BYTES)?;
        let chunk_padded =
            u32::from_le_bytes(header_bytes[0..4].try_into().unwrap()) as usize;
        let compressed_len =
            u32::from_le_bytes(header_bytes[4..8].try_into().unwrap()) as usize;
        if chunk_padded < HEADER_BYTES || chunk_padded < compressed_len {
            return Err(TsfBlobReaderError::UnexpectedLength {
                expected: chunk_padded,
                actual: compressed_len,
            });
        }
        Ok(TsfChunkHeader {
            _chunk_padded: chunk_padded,
            compressed_len,
        })
    }
}

#[derive(Debug)]
pub(crate) struct TsfSpectrumChunk {
    pub(crate) tof_indices: Vec<TofIndex>,
    pub(crate) intensities: Vec<f64>,
}

#[derive(Debug)]
struct TsfChunkHeader {
    _chunk_padded: usize,
    compressed_len: usize,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TsfBlobReaderError {
    #[error("{0}")]
    BinaryError(#[from] BinaryError),
    #[error("{0}")]
    TSFPathError(#[from] TSFPathError),
    #[error("IO error")]
    Io(#[from] io::Error),
    #[error("Unexpected length (expected at least {expected}, got {actual})")]
    UnexpectedLength { expected: usize, actual: usize },
    #[error("Integer overflow while computing peak payload size")]
    Overflow,
    #[error("Decompression failed")]
    Decompression(std::io::Error),
}
