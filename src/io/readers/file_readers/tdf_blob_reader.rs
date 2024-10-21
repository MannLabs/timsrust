mod tdf_blobs;

use memmap2::Mmap;
use std::fs::File;
use std::io;
use std::path::Path;
pub use tdf_blobs::*;
use zstd::decode_all;

const U32_SIZE: usize = std::mem::size_of::<u32>();
const HEADER_SIZE: usize = 2;

#[derive(Debug)]
pub struct TdfBlobReader {
    mmap: Mmap,
    global_file_offset: usize,
}

impl TdfBlobReader {
    // TODO parse compression1
    pub fn new(
        file_name: impl AsRef<Path>,
    ) -> Result<Self, TdfBlobReaderError> {
        let path = file_name.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let reader = Self {
            mmap,
            global_file_offset: 0,
        };
        Ok(reader)
    }

    pub fn get(&self, offset: usize) -> Result<TdfBlob, TdfBlobReaderError> {
        let offset = self.global_file_offset + offset;
        let byte_count = self
            .get_byte_count(offset)
            .ok_or(TdfBlobReaderError::InvalidOffset(offset))?;
        let compressed_bytes = self
            .get_compressed_bytes(offset, byte_count)
            .ok_or(TdfBlobReaderError::CorruptData)?;
        let bytes = decode_all(compressed_bytes)
            .map_err(|_| TdfBlobReaderError::Decompression)?;
        let blob = TdfBlob::new(bytes)?;
        Ok(blob)
    }

    fn get_byte_count(&self, offset: usize) -> Option<usize> {
        let start = offset as usize;
        let end = (offset + U32_SIZE) as usize;
        let raw_byte_count = self.mmap.get(start..end)?;
        let byte_count =
            u32::from_le_bytes(raw_byte_count.try_into().ok()?) as usize;
        Some(byte_count)
    }

    fn get_compressed_bytes(
        &self,
        offset: usize,
        byte_count: usize,
    ) -> Option<&[u8]> {
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
        file_name: impl AsRef<Path>,
        binary_offsets: Vec<usize>,
    ) -> Result<Self, IndexedTdfBlobReaderError> {
        let blob_reader = TdfBlobReader::new(file_name)?;
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
}

#[derive(Debug, thiserror::Error)]
pub enum IndexedTdfBlobReaderError {
    #[error("{0}")]
    TdfBlobReaderError(#[from] TdfBlobReaderError),
    #[cfg(feature = "minitdf")]
    #[error("Invalid index {0}")]
    InvalidIndex(usize),
}
