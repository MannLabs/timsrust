mod tdf_blobs;

use memmap2::Mmap;
use std::fs::File;
use std::io;
pub use tdf_blobs::*;
use zstd::decode_all;

use crate::readers::{TimsTofFileType, TimsTofPathError, TimsTofPathLike};

const U32_SIZE: usize = std::mem::size_of::<u32>();
const HEADER_SIZE: usize = 2;

#[derive(Debug)]
pub struct TdfBlobReader {
    bin_file_reader: TdfBinFileReader,
}

impl TdfBlobReader {
    pub fn new(path: impl TimsTofPathLike) -> Result<Self, TdfBlobReaderError> {
        let bin_file_reader = TdfBinFileReader::new(path)?;
        let reader = Self { bin_file_reader };
        Ok(reader)
    }

    pub fn get(&self, offset: usize) -> Result<TdfBlob, TdfBlobReaderError> {
        let offset = self.bin_file_reader.global_file_offset + offset;
        let byte_count = self
            .bin_file_reader
            .get_byte_count(offset)
            .ok_or(TdfBlobReaderError::InvalidOffset(offset))?;
        let data = self
            .bin_file_reader
            .get_data(offset, byte_count)
            .ok_or(TdfBlobReaderError::CorruptData)?;
        let bytes =
            decode_all(data).map_err(|_| TdfBlobReaderError::Decompression)?;
        let blob = TdfBlob::new(bytes)?;
        Ok(blob)
    }
}

#[derive(Debug)]
struct TdfBinFileReader {
    mmap: Mmap,
    global_file_offset: usize,
}

impl TdfBinFileReader {
    // TODO parse compression1
    fn new(path: impl TimsTofPathLike) -> Result<Self, TdfBlobReaderError> {
        let path = path.to_timstof_path()?;
        let bin_path = match path.file_type() {
            #[cfg(feature = "tdf")]
            TimsTofFileType::TDF => path.tdf_bin()?,
            #[cfg(feature = "minitdf")]
            TimsTofFileType::MiniTDF => path.ms2_bin()?,
        };
        let file = File::open(bin_path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let reader = Self {
            mmap,
            global_file_offset: 0,
        };
        Ok(reader)
    }

    fn get_byte_count(&self, offset: usize) -> Option<usize> {
        let start = offset as usize;
        let end = start + U32_SIZE as usize;
        let raw_byte_count = self.mmap.get(start..end)?;
        let byte_count =
            u32::from_le_bytes(raw_byte_count.try_into().ok()?) as usize;
        Some(byte_count)
    }

    // fn get_scan_count(&self, offset: usize) -> Option<usize> {
    //     let start = (offset + U32_SIZE) as usize;
    //     let end = start + U32_SIZE as usize;
    //     let raw_scan_count = self.mmap.get(start..end)?;
    //     let scan_count =
    //         u32::from_le_bytes(raw_scan_count.try_into().ok()?) as usize;
    //     Some(scan_count)
    // }

    fn get_data(&self, offset: usize, byte_count: usize) -> Option<&[u8]> {
        let start = offset + HEADER_SIZE * U32_SIZE;
        let end = offset + byte_count;
        self.mmap.get(start..end)
    }
}

#[cfg(feature = "minitdf")]
#[derive(Debug)]
pub struct IndexedTdfBlobReader {
    blob_reader: TdfBlobReader,
    binary_offsets: Vec<usize>,
}

#[cfg(feature = "minitdf")]
impl IndexedTdfBlobReader {
    pub fn new(
        path: impl TimsTofPathLike,
        binary_offsets: Vec<usize>,
    ) -> Result<Self, IndexedTdfBlobReaderError> {
        let blob_reader = TdfBlobReader::new(path)?;
        let reader = Self {
            binary_offsets,
            blob_reader: blob_reader,
        };
        Ok(reader)
    }

    pub fn get(
        &self,
        index: usize,
    ) -> Result<TdfBlob, IndexedTdfBlobReaderError> {
        let offset = *self
            .binary_offsets
            .get(index)
            .ok_or(IndexedTdfBlobReaderError::InvalidIndex(index))?;
        let blob = self.blob_reader.get(offset)?;
        Ok(blob)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TdfBlobReaderError {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("{0}")]
    TdfBlob(#[from] TdfBlobError),
    #[error("Data is corrupt")]
    CorruptData,
    #[error("Decompression fails")]
    Decompression,
    #[error("Invalid offset {0}")]
    InvalidOffset(usize),
    #[error("{0}")]
    TimsTofPathError(#[from] TimsTofPathError),
    #[error("No binary file found")]
    NoBinary,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexedTdfBlobReaderError {
    #[error("{0}")]
    TdfBlobReaderError(#[from] TdfBlobReaderError),
    #[cfg(feature = "minitdf")]
    #[error("Invalid index {0}")]
    InvalidIndex(usize),
}
